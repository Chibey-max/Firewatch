//! LND over its REST API, authenticated with `readonly.macaroon`.
//!
//! Firewatch only reads `GetInfo`, and refuses a macaroon that can write.
//! Most `lnd.conf` options are not exposed over RPC, so the node's config is
//! reported as unknown and config conditions come back "Can't tell".

use std::path::PathBuf;

use firewatch_core::{Implementation, NodeInfo};
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
}

pub struct LndClient {
    http: reqwest::Client,
    name: String,
    url: String,
    macaroon: String,
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
            url: config.url.trim_end_matches('/').to_string(),
            macaroon,
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
            .map_err(|e| crate::transport_error(e, &self.url, &self.name))?;
        let ok = response.status().is_success();
        let body: Value = response
            .json()
            .await
            .map_err(|e| crate::transport_error(e, &self.url, &self.name))?;
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
        parse_getinfo(&self.name, &info)
    }
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

    #[test]
    fn missing_version_is_an_error() {
        assert!(parse_getinfo("alice", &json!({})).is_err());
    }
}
