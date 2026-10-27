//! Core Lightning over its REST API (`clnrest`), authenticated with a rune.
//!
//! Firewatch only calls `getinfo` and `listconfigs`, and refuses a rune that
//! can call anything else, so a stolen Firewatch config cannot move funds or
//! change settings. `listconfigs` is limited to the four options Firewatch
//! reads: asked for every option, Core Lightning before 26.06.9 returns
//! secrets such as bitcoind's RPC password and a `--recover` seed. Create the
//! rune on the node with:
//!
//! ```sh
//! lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"],["method/listconfigs","pnameconfig=clnrest-host","pnameconfig=clnrest-port","pnameconfig=grpc-host","pnameconfig=grpc-port"]]'
//! ```

use std::path::PathBuf;

use firewatch_core::{Api, ApiListen, Implementation, Listen, NodeInfo};
use serde::Deserialize;
use serde_json::Value;

use crate::ConnectorError;

/// CLN's error code for a rune it doesn't recognise.
const NOT_AUTHORIZED: i64 = 1501;
/// CLN's error code for a call the rune does not permit.
const NOT_PERMITTED: i64 = 1502;

/// CLN's error code for an option the node doesn't have, e.g. `grpc-port`
/// when cln-grpc is disabled.
const UNKNOWN_OPTION: i64 = -32602;

/// The rune Firewatch asks for: `getinfo`, and `listconfigs` for
/// [`API_OPTIONS`] only. A rule on `pnameconfig` fails when `config` is
/// missing, so a plain `listconfigs` (every option) is refused.
pub const RUNE_RESTRICTIONS: &str = r#"[["method=getinfo","method=listconfigs"],["method/listconfigs","pnameconfig=clnrest-host","pnameconfig=clnrest-port","pnameconfig=grpc-host","pnameconfig=grpc-port"]]"#;

/// The options Firewatch reads: where clnrest and cln-grpc listen.
const API_OPTIONS: [&str; 4] = ["clnrest-host", "clnrest-port", "grpc-host", "grpc-port"];

/// An option every node has and [`RUNE_RESTRICTIONS`] refuses. A rune that
/// can read it can read every option: the older, wider rune.
const WIDE_PROBE_OPTION: &str = "alias";

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
    /// Allow plain `http://` to another machine. Off by default: the rune
    /// would cross the network unencrypted. For a connection that is already
    /// private, such as a Docker container on this machine or a VPN.
    #[serde(default)]
    pub allow_plain_http: bool,
}

pub struct ClnClient {
    http: reqwest::Client,
    name: String,
    url: String,
    rune: String,
    /// Set when `allow_plain_http` let a remote `http://` url through.
    plain_http: Option<String>,
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
        let url = crate::node_url(&config.name, &config.url)?;
        let plain_http = crate::plain_http_check(
            &config.name,
            &url,
            Implementation::Cln,
            config.allow_plain_http,
        )?;
        Ok(ClnClient {
            http,
            name: config.name.clone(),
            url,
            rune: rune.trim().to_string(),
            plain_http,
        })
    }

    async fn call(&self, method: &str) -> Result<Value, ConnectorError> {
        self.call_with(method, serde_json::json!({})).await
    }

    /// `params` is sent as JSON: clnrest ignores a body without that content
    /// type, and the rune then sees no parameters.
    async fn call_with(&self, method: &str, params: Value) -> Result<Value, ConnectorError> {
        let response = self
            .http
            .post(format!("{}/v1/{method}", self.url))
            .header("Rune", &self.rune)
            .json(&params)
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
        let (options, refused) = self.read_options().await?;
        let mut warnings: Vec<String> = self.plain_http.iter().cloned().collect();
        if self.rune_reads_every_option().await? {
            warnings.push(format!(
                "the rune for '{}' can read every config option. On Core Lightning before 26.06.9 that includes \
                 bitcoind's RPC password and a --recover seed (Firewatch never reads them, but anyone who copies \
                 the rune can). Create a narrower one with `lightning-cli createrune restrictions='{RUNE_RESTRICTIONS}'`, \
                 put it in the config, then retire the old one with `lightning-cli blacklistrune start=<its unique_id>`",
                self.name
            ));
        }

        Ok(NodeInfo {
            name: self.name.clone(),
            implementation: Implementation::Cln,
            version,
            network: info["network"].as_str().map(str::to_string),
            // Firewatch's rune can't read the node's flags, so a
            // `config_enabled` condition is "Can't tell", never "off".
            config: None,
            features: None,
            commit_hash: None,
            apis: parse_apis(&options, &refused),
            warnings,
        })
    }

    /// Each of [`API_OPTIONS`], asked for by name. Returns what the node
    /// reported, as one `listconfigs`-shaped object, and the options the rune
    /// refused. An option the node doesn't have is simply left out.
    async fn read_options(&self) -> Result<(Value, Vec<&'static str>), ConnectorError> {
        let reads =
            futures::future::join_all(API_OPTIONS.map(|option| {
                self.call_with("listconfigs", serde_json::json!({ "config": option }))
            }))
            .await;
        let mut configs = serde_json::Map::new();
        let mut refused = Vec::new();
        for (option, read) in API_OPTIONS.into_iter().zip(reads) {
            match read {
                Ok(body) => {
                    if let Some(entry) = body["configs"].get(option) {
                        configs.insert(option.to_string(), entry.clone());
                    }
                }
                Err(ConnectorError::Rpc { code, .. }) if code == UNKNOWN_OPTION => {}
                Err(ConnectorError::Rpc { .. }) => refused.push(option),
                Err(e) => return Err(e),
            }
        }
        Ok((serde_json::json!({ "configs": configs }), refused))
    }

    /// Whether the rune can read an option Firewatch's rune refuses. Only the
    /// harmless `alias` is asked for, and its value is dropped.
    async fn rune_reads_every_option(&self) -> Result<bool, ConnectorError> {
        let probe = serde_json::json!({ "config": WIDE_PROBE_OPTION });
        match self.call_with("listconfigs", probe).await {
            Ok(_) => Ok(true),
            Err(ConnectorError::Rpc { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

/// Where clnrest and cln-grpc listen, from their `listconfigs` options. REST
/// is always there: Firewatch just read the node through it. gRPC is left
/// out when it has no port, which means the plugin isn't serving. An API
/// whose options the rune refused is "unknown".
fn parse_apis(listconfigs: &Value, refused: &[&str]) -> Vec<ApiListen> {
    let configs = &listconfigs["configs"];
    [(Api::Rest, "clnrest"), (Api::Grpc, "grpc")]
        .into_iter()
        .filter_map(|(api, prefix)| {
            let (host_option, port_option) = (format!("{prefix}-host"), format!("{prefix}-port"));
            if refused.contains(&host_option.as_str()) || refused.contains(&port_option.as_str()) {
                return Some(ApiListen {
                    api,
                    listen: Listen::Unknown {
                        why: format!(
                            "the rune doesn't allow reading {host_option} and {port_option}"
                        ),
                    },
                });
            }
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
    fn reads_where_clnrest_and_grpc_listen() {
        // As Core Lightning 25.12 returned these options, one call each, run
        // with --clnrest-host=0.0.0.0 (as Polar does).
        let listconfigs = serde_json::json!({ "configs": {
            "clnrest-host": { "plugin": "/usr/local/libexec/c-lightning/plugins/clnrest", "source": "cmdline", "value_str": "0.0.0.0" },
            "clnrest-port": { "plugin": "/usr/local/libexec/c-lightning/plugins/clnrest", "source": "cmdline", "value_int": 8080 },
            "grpc-host": { "plugin": "/usr/local/libexec/c-lightning/plugins/cln-grpc", "source": "default", "value_str": "127.0.0.1" },
            "grpc-port": { "plugin": "/usr/local/libexec/c-lightning/plugins/cln-grpc", "source": "default", "value_int": 9736 }
        }});
        assert_eq!(
            parse_apis(&listconfigs, &[]),
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
            parse_apis(&listconfigs, &[]),
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
            parse_apis(&listconfigs, &[])[0].listen,
            Listen::Read {
                addresses: vec!["[::]:3010".into()]
            }
        );
    }

    #[test]
    fn missing_clnrest_options_are_unknown() {
        let apis = parse_apis(&serde_json::json!({ "configs": {} }), &[]);
        assert_eq!(apis.len(), 1);
        assert!(matches!(apis[0].listen, Listen::Unknown { .. }));
    }

    #[test]
    fn requires_exactly_one_rune_source() {
        let config = ClnConfig {
            name: "n".into(),
            url: "http://x".into(),
            rune: None,
            rune_file: None,
            tls_cert: None,
            allow_plain_http: false,
        };
        assert!(matches!(
            ClnClient::new(&config, reqwest::Client::new()),
            Err(ConnectorError::Config(_))
        ));
    }

    #[test]
    fn a_refused_api_is_unknown() {
        let listconfigs = serde_json::json!({ "configs": {
            "clnrest-host": { "value_str": "127.0.0.1" },
            "clnrest-port": { "value_int": 3010 }
        }});
        let apis = parse_apis(&listconfigs, &["grpc-host", "grpc-port"]);
        assert_eq!(apis.len(), 2);
        assert_eq!(
            apis[1],
            ApiListen {
                api: Api::Grpc,
                listen: Listen::Unknown {
                    why: "the rune doesn't allow reading grpc-host and grpc-port".into()
                }
            }
        );
    }

    #[test]
    fn the_rune_allows_exactly_the_options_firewatch_reads() {
        let rules: Vec<Vec<String>> = serde_json::from_str(RUNE_RESTRICTIONS).unwrap();
        assert_eq!(rules[0], ["method=getinfo", "method=listconfigs"]);
        let mut allowed: Vec<&str> = rules[1]
            .iter()
            .filter_map(|r| r.strip_prefix("pnameconfig="))
            .collect();
        allowed.sort();
        let mut read = API_OPTIONS.to_vec();
        read.sort();
        assert_eq!(allowed, read);
        assert!(!allowed.contains(&WIDE_PROBE_OPTION));
        assert_eq!(rules.len(), 2);
    }
}
