//! LND over its REST API, authenticated with `readonly.macaroon`.
//!
//! Firewatch only reads `GetInfo`, and refuses a macaroon that can write.
//! Most `lnd.conf` options are not exposed over RPC, so the node's config is
//! reported as unknown and config conditions come back "Can't tell".

use std::path::{Path, PathBuf};

use firewatch_core::{Api, ApiListen, Implementation, Listen, NodeInfo};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{tls, ConnectorError};

/// LND's gRPC code for "permission denied", as returned by its REST proxy.
const PERMISSION_DENIED: i64 = 2;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LndConfig {
    pub name: String,
    /// REST base URL, e.g. `https://127.0.0.1:8081`.
    pub url: String,
    /// Path to `readonly.macaroon`.
    pub macaroon_file: PathBuf,
    /// LND's `tls.cert`, trusted for this node only.
    #[serde(default)]
    pub tls_cert: Option<PathBuf>,
    /// LND's `lnd.conf`, for the exposure check: LND doesn't report where
    /// its APIs listen over RPC. Read on this machine.
    #[serde(default)]
    pub lnd_conf: Option<PathBuf>,
}

pub struct LndClient {
    http: reqwest::Client,
    name: String,
    url: String,
    macaroon: String,
    lnd_conf: Option<PathBuf>,
}

impl LndClient {
    pub fn new(config: &LndConfig) -> Result<Self, ConnectorError> {
        let read = |path: &PathBuf| {
            std::fs::read(path)
                .map_err(|e| ConnectorError::Config(format!("reading {}: {e}", path.display())))
        };
        let macaroon: String = read(&config.macaroon_file)?
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        let mut http = reqwest::Client::builder().timeout(crate::TIMEOUT);
        if let Some(cert) = &config.tls_cert {
            http = http.tls_backend_preconfigured(tls::pinned_config(&read(cert)?)?);
        }
        Ok(LndClient {
            http: http
                .build()
                .map_err(|e| ConnectorError::Config(e.to_string()))?,
            name: config.name.clone(),
            url: crate::node_url(&config.name, &config.url)?,
            macaroon,
            lnd_conf: config.lnd_conf.clone(),
        })
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> Result<Value, ConnectorError> {
        let response = request
            .header("Grpc-Metadata-macaroon", &self.macaroon)
            .send()
            .await
            .map_err(|e| crate::transport_error(e, &self.url, &self.name, Implementation::Lnd))?;
        let ok = response.status().is_success();
        let body = crate::read_json(response, &self.url, &self.name, Implementation::Lnd).await?;
        if ok {
            return Ok(body);
        }
        Err(ConnectorError::Rpc {
            method: path.to_string(),
            code: body["code"].as_i64().unwrap_or_default(),
            message: body["message"].as_str().unwrap_or("no message").to_string(),
        })
    }

    /// Fails unless the macaroon is refused a write permission. Probes
    /// `SignMessage` (needs `message:write`), which changes nothing on the node.
    pub async fn ensure_read_only(&self) -> Result<(), ConnectorError> {
        let path = "/v1/signmessage";
        let probe = self
            .http
            .post(format!("{}{path}", self.url))
            .json(&json!({ "msg": "ZmlyZXdhdGNoLXByb2Jl" }));
        match self.send(probe, path).await {
            Err(ConnectorError::Rpc { code, .. }) if code == PERMISSION_DENIED => Ok(()),
            Ok(_) => Err(ConnectorError::NotReadOnly(format!(
                "the macaroon for '{}' can sign messages; Firewatch only accepts readonly.macaroon",
                self.name
            ))),
            Err(e) => Err(e),
        }
    }

    pub async fn node_info(&self) -> Result<NodeInfo, ConnectorError> {
        let path = "/v1/getinfo";
        // A macaroon from another node fails verification; one without
        // info:read is denied. Either way the credential is the problem.
        let info = self
            .send(self.http.get(format!("{}{path}", self.url)), path)
            .await
            .map_err(|e| match e {
                ConnectorError::Rpc { message, .. }
                    if message.contains("verification failed")
                        || message.contains("macaroon")
                        || message.contains("permission denied") =>
                {
                    ConnectorError::CredentialRejected(message)
                }
                other => other,
            })?;
        self.ensure_read_only().await?;
        let mut node = parse_getinfo(&self.name, &info)?;
        node.apis = listen_from_conf(self.lnd_conf.as_deref());
        Ok(node)
    }
}

/// Where LND's REST and gRPC APIs listen, from `lnd.conf`. Flags on LND's
/// command line override the file and can't be seen from here, so an option
/// the file doesn't set is unknown, not LND's localhost default.
fn listen_from_conf(path: Option<&Path>) -> Vec<ApiListen> {
    let unknown = |why: String| {
        [Api::Rest, Api::Grpc]
            .map(|api| ApiListen {
                api,
                listen: Listen::Unknown { why: why.clone() },
            })
            .to_vec()
    };
    let Some(path) = path else {
        return unknown("LND doesn't report it over RPC; set lnd_conf to read lnd.conf".into());
    };
    match std::fs::read_to_string(path) {
        Ok(text) => parse_lnd_conf(&text),
        Err(e) => unknown(format!("reading {}: {e}", path.display())),
    }
}

/// `restlisten` and `rpclisten` from lnd.conf. Each may appear more than once.
/// Lines starting with `;` or `#` are comments.
fn parse_lnd_conf(text: &str) -> Vec<ApiListen> {
    let values = |key: &str| -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.starts_with(';') && !line.starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .filter(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect()
    };
    [(Api::Rest, "restlisten"), (Api::Grpc, "rpclisten")]
        .map(|(api, key)| {
            let addresses = values(key);
            let listen = if addresses.is_empty() {
                Listen::Unknown {
                    why: format!(
                        "lnd.conf doesn't set {key}, and LND's command line can't be read"
                    ),
                }
            } else {
                Listen::Read { addresses }
            };
            ApiListen { api, listen }
        })
        .to_vec()
}

fn parse_getinfo(name: &str, info: &Value) -> Result<NodeInfo, ConnectorError> {
    let version = info["version"]
        .as_str()
        .ok_or_else(|| ConnectorError::Unexpected("getinfo has no version".into()))?
        .to_string();
    let features = info["features"]
        .as_object()
        .map(|f| f.keys().filter_map(|bit| bit.parse().ok()).collect());
    Ok(NodeInfo {
        name: name.to_string(),
        implementation: Implementation::Lnd,
        version,
        network: info["chains"][0]["network"].as_str().map(str::to_string),
        config: None,
        features,
        commit_hash: info["commit_hash"]
            .as_str()
            .filter(|h| !h.is_empty())
            .map(str::to_string),
        apis: vec![],
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn parses_getinfo() {
        let info = json!({
            "version": "0.20.0-beta commit=v0.20.0-beta",
            "commit_hash": "b9ea7070c20ad2ca8514a47d9b4d560a501f0487",
            "chains": [{ "chain": "bitcoin", "network": "regtest" }],
            "features": {
                "0": { "name": "data-loss-protect", "is_required": true },
                "5": { "name": "upfront-shutdown-script", "is_required": false }
            }
        });
        let node = parse_getinfo("alice", &info).unwrap();
        assert_eq!(node.version, "0.20.0-beta commit=v0.20.0-beta");
        assert_eq!(node.network.as_deref(), Some("regtest"));
        assert_eq!(node.features, Some(BTreeSet::from([0, 5])));
        assert!(node.config.is_none(), "lnd.conf is not readable over RPC");
        assert_eq!(
            node.commit_hash.as_deref(),
            Some("b9ea7070c20ad2ca8514a47d9b4d560a501f0487")
        );
    }

    fn addresses(apis: &[ApiListen], api: Api) -> Listen {
        apis.iter().find(|a| a.api == api).unwrap().listen.clone()
    }

    #[test]
    fn reads_restlisten_and_rpclisten_from_lnd_conf() {
        let conf = "\
[Application Options]
; restlisten=127.0.0.1:8080
# rpclisten=127.0.0.1:10009
restlisten = 0.0.0.0:8080
restlisten=localhost:8081
rpclisten=:10009
";
        let apis = parse_lnd_conf(conf);
        assert_eq!(
            addresses(&apis, Api::Rest),
            Listen::Read {
                addresses: vec!["0.0.0.0:8080".into(), "localhost:8081".into()]
            }
        );
        assert_eq!(
            addresses(&apis, Api::Grpc),
            Listen::Read {
                addresses: vec![":10009".into()]
            }
        );
    }

    #[test]
    fn an_option_lnd_conf_doesnt_set_is_unknown() {
        // Not LND's localhost default: a command-line flag may override it.
        let apis = parse_lnd_conf("[Application Options]\nalias=alice\n");
        for api in [Api::Rest, Api::Grpc] {
            assert!(matches!(addresses(&apis, api), Listen::Unknown { .. }));
        }
    }

    #[test]
    fn without_lnd_conf_or_with_an_unreadable_one_both_apis_are_unknown() {
        for apis in [
            listen_from_conf(None),
            listen_from_conf(Some(Path::new("/nonexistent/lnd.conf"))),
        ] {
            assert_eq!(apis.len(), 2);
            assert!(apis
                .iter()
                .all(|a| matches!(a.listen, Listen::Unknown { .. })));
        }
    }

    #[test]
    fn missing_version_is_an_error() {
        assert!(parse_getinfo("alice", &json!({})).is_err());
    }
}
