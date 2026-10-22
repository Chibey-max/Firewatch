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

use firewatch_core::{Implementation, NodeInfo};
use serde::Deserialize;
use serde_json::Value;

use crate::ConnectorError;

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
        Ok(ClnClient {
            http,
            name: config.name.clone(),
            url: config.url.trim_end_matches('/').to_string(),
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
            .await?;
        let ok = response.status().is_success();
        let body: Value = response.json().await?;
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
        let info = self.call("getinfo").await?;
        self.ensure_read_only().await?;

        let version = info["version"]
            .as_str()
            .ok_or_else(|| ConnectorError::Unexpected("getinfo has no version".into()))?
            .to_string();
        // A config we fail to read is "unknown", never "all options off".
        let config = match self.call("listconfigs").await {
            Ok(configs) => Some(parse_configs(&configs)),
            Err(ConnectorError::Rpc { .. }) => None,
            Err(e) => return Err(e),
        };

        Ok(NodeInfo {
            name: self.name.clone(),
            implementation: Implementation::Cln,
            version,
            network: info["network"].as_str().map(str::to_string),
            config,
            features: None,
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
        };
        assert!(matches!(
            ClnClient::new(&config, reqwest::Client::new()),
            Err(ConnectorError::Config(_))
        ));
    }
}
