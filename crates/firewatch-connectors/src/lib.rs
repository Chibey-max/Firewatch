//! Read-only node connectors. Each one turns a node into a
//! [`firewatch_core::NodeInfo`] using credentials that cannot change it.

use std::path::Path;
use std::time::Duration;

use firewatch_core::{Implementation, NodeInfo};
use serde::Deserialize;
use thiserror::Error;

pub mod cln;
pub mod lnd;
mod tls;

pub use cln::{ClnClient, ClnConfig};
pub use lnd::{LndClient, LndConfig};

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("config: {0}")]
    Config(String),
    #[error("request failed: {}", error_chain(.0))]
    Http(#[from] reqwest::Error),
    #[error("{method} refused (code {code}): {message}")]
    Rpc {
        method: String,
        code: i64,
        message: String,
    },
    #[error("credential is not read-only: {0}")]
    NotReadOnly(String),
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

/// `firewatch.toml`: the nodes to watch.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub nodes: Vec<NodeConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum NodeConfig {
    Cln(ClnConfig),
    Lnd(LndConfig),
}

impl NodeConfig {
    pub fn name(&self) -> &str {
        match self {
            NodeConfig::Cln(c) => &c.name,
            NodeConfig::Lnd(c) => &c.name,
        }
    }

    pub fn implementation(&self) -> Implementation {
        match self {
            NodeConfig::Cln(_) => Implementation::Cln,
            NodeConfig::Lnd(_) => Implementation::Lnd,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConnectorError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ConnectorError::Config(format!("reading {}: {e}", path.display())))?;
        toml::from_str(&text)
            .map_err(|e| ConnectorError::Config(format!("parsing {}: {e}", path.display())))
    }
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("static reqwest config is valid")
}

/// Reads one node.
pub async fn fetch(node: &NodeConfig, http: &reqwest::Client) -> Result<NodeInfo, ConnectorError> {
    match node {
        NodeConfig::Cln(config) => ClnClient::new(config, http.clone())?.node_info().await,
        NodeConfig::Lnd(config) => LndClient::new(config)?.node_info().await,
    }
}

/// reqwest's own message hides the cause (TLS, refused, timeout); include it.
fn error_chain(e: &reqwest::Error) -> String {
    let mut out = e.to_string();
    let mut source = std::error::Error::source(e);
    while let Some(cause) = source {
        out.push_str(": ");
        out.push_str(&cause.to_string());
        source = cause.source();
    }
    out
}
