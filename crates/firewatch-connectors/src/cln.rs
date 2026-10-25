//! Core Lightning over its REST API (`clnrest`), authenticated with a rune.
//!
//! Firewatch only calls `getinfo` and `listconfigs`, and refuses a rune that
//! can call anything else, so a stolen Firewatch config cannot move funds or
//! change settings. Create the rune on the node with:
//!
//! ```sh
//! lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"]]'
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use firewatch_core::{Api, ApiListen, Implementation, Listen, NodeInfo};
use serde::Deserialize;
use serde_json::Value;

use crate::ConnectorError;

/// CLN's error code for a rune it doesn't recognise.
const NOT_AUTHORIZED: i64 = 1501;
/// CLN's error code for a call the rune does not permit.
const NOT_PERMITTED: i64 = 1502;

/// A method the rune must NOT allow. Read-only itself, so probing it is
/// harmless even when the rune turns out to be unrestricted.
const PROBE_METHOD: &str = "listfunds";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClnConfig {
    pub name: String,
    /// clnrest base URL, e.g. `http://127.0.0.1:8182`.
    pub url: String,
    /// The rune itself, or `rune_file` with a path to it.
    #[serde(default)]
    pub rune: Option<String>,
    #[serde(default)]
    pub rune_file: Option<PathBuf>,
    /// For clnrest over HTTPS: its self-signed `server.pem` (in the node's
    /// network folder), trusted for this node only. Not needed over HTTP.
    #[serde(default)]
    pub tls_cert: Option<PathBuf>,
}

pub struct ClnClient {
    http: reqwest::Client,
    name: String,
    url: String,
    rune: String,
}

impl ClnClient {
    pub fn new(config: &ClnConfig, http: reqwest::Client) -> Result<Self, ConnectorError> {
        let rune = match (&config.rune, &config.rune_file) {
            (Some(rune), None) => rune.clone(),
            (None, Some(path)) => std::fs::read_to_string(path).map_err(|e| {
                ConnectorError::Config(format!("reading rune_file {}: {e}", path.display()))
            })?,
            _ => {
                return Err(ConnectorError::Config(format!(
                    "node '{}': set exactly one of rune or rune_file",
                    config.name
                )))
            }
        };
        let http = match &config.tls_cert {
            None => http,
            Some(path) => {
                let pem = std::fs::read(path).map_err(|e| {
                    ConnectorError::Config(format!("reading tls_cert {}: {e}", path.display()))
                })?;
                reqwest::Client::builder()
                    .timeout(crate::TIMEOUT)
                    .tls_backend_preconfigured(crate::tls::pinned_config(&pem)?)
                    .build()
                    .map_err(|e| ConnectorError::Config(e.to_string()))?
            }
        };
        Ok(ClnClient {
            http,
            name: config.name.clone(),
            url: crate::node_url(&config.name, &config.url)?,
            rune: rune.trim().to_string(),
        })
    }

    async fn call(&self, method: &str) -> Result<Value, ConnectorError> {
        let response = self
            .http
            .post(format!("{}/v1/{method}", self.url))
            .header("Rune", &self.rune)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| crate::transport_error(e, &self.url, &self.name, Implementation::Cln))?;
        let ok = response.status().is_success();
        let body = crate::read_json(response, &self.url, &self.name, Implementation::Cln).await?;
        if ok {
            return Ok(body);
        }
        Err(ConnectorError::Rpc {
            method: method.to_string(),
            code: body["code"].as_i64().unwrap_or_default(),
            message: body["message"].as_str().unwrap_or("no message").to_string(),
        })
    }

    /// Fails unless the rune is refused for anything beyond reading.
    pub async fn ensure_read_only(&self) -> Result<(), ConnectorError> {
        match self.call(PROBE_METHOD).await {
            Err(ConnectorError::Rpc { code, .. }) if code == NOT_PERMITTED => Ok(()),
            Ok(_) => Err(ConnectorError::NotReadOnly(format!(
                "the rune for '{}' can call {PROBE_METHOD}; Firewatch only accepts a rune restricted to getinfo and listconfigs",
                self.name
            ))),
            Err(e) => Err(e),
        }
    }

    pub async fn node_info(&self) -> Result<NodeInfo, ConnectorError> {
        // A rune this node didn't issue fails with 1501; one that doesn't allow
        // getinfo fails with 1502. Either way the credential is the problem.
        let info = self.call("getinfo").await.map_err(|e| match e {
            ConnectorError::Rpc { code, message, .. }
                if code == NOT_AUTHORIZED || code == NOT_PERMITTED =>
            {
                ConnectorError::CredentialRejected(message)
            }
            other => other,
        })?;
        self.ensure_read_only().await?;

        let version = info["version"]
            .as_str()
            .ok_or_else(|| ConnectorError::Unexpected("getinfo has no version".into()))?
            .to_string();
        // A config we fail to read is "unknown", never "all options off".
        let (config, apis) = match self.call("listconfigs").await {
            Ok(configs) => (Some(parse_configs(&configs)), parse_apis(&configs)),
            Err(ConnectorError::Rpc { message, .. }) => (
                None,
                vec![ApiListen {
                    api: Api::Rest,
                    listen: Listen::Unknown {
                        why: format!("listconfigs was refused ({message})"),
                    },
                }],
            ),
            Err(e) => return Err(e),
        };

        Ok(NodeInfo {
            name: self.name.clone(),
            implementation: Implementation::Cln,
            version,
            network: info["network"].as_str().map(str::to_string),
            config,
            features: None,
            commit_hash: None,
            apis,
        })
    }
}

/// Maps `listconfigs` to option -> enabled. Only flags (`set`) and boolean
/// options (`value_bool`) are included; anything else is left out, so a
/// condition on it comes back "Can't tell" rather than guessed.
fn parse_configs(listconfigs: &Value) -> BTreeMap<String, bool> {
    let Some(configs) = listconfigs["configs"].as_object() else {
        return BTreeMap::new();
    };
    configs
        .iter()
        .filter_map(|(name, entry)| {
            entry["set"]
                .as_bool()
                .or_else(|| entry["value_bool"].as_bool())
                .map(|enabled| (name.clone(), enabled))
        })
        .collect()
}

/// Where clnrest and cln-grpc listen, from their `listconfigs` options. REST
/// is always there: Firewatch just read the node through it. gRPC is left
/// out when it has no port, which means the plugin isn't serving.
fn parse_apis(listconfigs: &Value) -> Vec<ApiListen> {
    let configs = &listconfigs["configs"];
    [(Api::Rest, "clnrest"), (Api::Grpc, "grpc")]
        .into_iter()
        .filter_map(|(api, prefix)| {
            let host = configs[format!("{prefix}-host")]["value_str"].as_str();
            let port = configs[format!("{prefix}-port")]["value_int"].as_i64();
            let listen = match (host, port) {
                (Some(host), Some(port)) => Listen::Read {
                    addresses: vec![join_host_port(host, port)],
                },
                (_, None) if api == Api::Grpc => return None,
                _ => Listen::Unknown {
                    why: format!("listconfigs has no {prefix}-host and {prefix}-port"),
                },
            };
            Some(ApiListen { api, listen })
        })
        .collect()
}

/// `0.0.0.0:8080`, or `[::]:8080` for an IPv6 host. clnrest builds its address
/// as `format!("{host}:{port}")`, so an IPv6 host is set as `[::]` already.
fn join_host_port(host: &str, port: i64) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flags_and_bools_only() {
        let listconfigs = serde_json::json!({
            "configs": {
                "experimental-splicing": { "set": false, "source": "default" },
                "developer": { "set": true, "source": "cmdline" },
                "allow-deprecated-apis": { "value_bool": true, "source": "default" },
                "alias": { "value_str": "old-cln", "source": "cmdline" },
                "addr": { "values_str": ["old-cln"], "sources": ["cmdline"] }
            }
        });
        let parsed = parse_configs(&listconfigs);
        assert_eq!(
            parsed,
            BTreeMap::from([
                ("allow-deprecated-apis".to_string(), true),
                ("developer".to_string(), true),
                ("experimental-splicing".to_string(), false),
            ])
        );
    }

    #[test]
    fn reads_where_clnrest_and_grpc_listen() {
        // As Core Lightning 25.12 returned it to a getinfo/listconfigs rune,
        // run with --clnrest-host=0.0.0.0 (as Polar does).
        let listconfigs = serde_json::json!({ "configs": {
            "clnrest-host": { "plugin": "/usr/local/libexec/c-lightning/plugins/clnrest", "source": "cmdline", "value_str": "0.0.0.0" },
            "clnrest-port": { "plugin": "/usr/local/libexec/c-lightning/plugins/clnrest", "source": "cmdline", "value_int": 8080 },
            "grpc-host": { "plugin": "/usr/local/libexec/c-lightning/plugins/cln-grpc", "source": "default", "value_str": "127.0.0.1" },
            "grpc-port": { "plugin": "/usr/local/libexec/c-lightning/plugins/cln-grpc", "source": "default", "value_int": 9736 }
        }});
        assert_eq!(
            parse_apis(&listconfigs),
            vec![
                ApiListen {
                    api: Api::Rest,
                    listen: Listen::Read {
                        addresses: vec!["0.0.0.0:8080".into()]
                    }
                },
                ApiListen {
                    api: Api::Grpc,
                    listen: Listen::Read {
                        addresses: vec!["127.0.0.1:9736".into()]
                    }
                },
            ]
        );
    }

    #[test]
    fn grpc_without_a_port_isnt_serving_and_ipv6_is_bracketed() {
        let listconfigs = serde_json::json!({ "configs": {
            "clnrest-host": { "value_str": "::" },
            "clnrest-port": { "value_int": 3010 }
        }});
        assert_eq!(
            parse_apis(&listconfigs),
            vec![ApiListen {
                api: Api::Rest,
                listen: Listen::Read {
                    addresses: vec!["[::]:3010".into()]
                }
            }]
        );
    }

    #[test]
    fn a_bracketed_ipv6_host_is_not_bracketed_twice() {
        // clnrest only starts with an IPv6 host written as `[::]`.
        let listconfigs = serde_json::json!({ "configs": {
            "clnrest-host": { "value_str": "[::]" },
            "clnrest-port": { "value_int": 3010 }
        }});
        assert_eq!(
            parse_apis(&listconfigs)[0].listen,
            Listen::Read {
                addresses: vec!["[::]:3010".into()]
            }
        );
    }

    #[test]
    fn missing_clnrest_options_are_unknown() {
        let apis = parse_apis(&serde_json::json!({ "configs": {} }));
        assert_eq!(apis.len(), 1);
        assert!(matches!(apis[0].listen, Listen::Unknown { .. }));
    }

    #[test]
    fn missing_configs_key_gives_empty_map() {
        assert!(parse_configs(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn requires_exactly_one_rune_source() {
        let config = ClnConfig {
            name: "n".into(),
            url: "http://x".into(),
            rune: None,
            rune_file: None,
            tls_cert: None,
        };
        assert!(matches!(
            ClnClient::new(&config, reqwest::Client::new()),
            Err(ConnectorError::Config(_))
        ));
    }
}
