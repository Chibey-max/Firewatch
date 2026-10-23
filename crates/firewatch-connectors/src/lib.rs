//! Read-only node connectors. Each one turns a node into a
//! [`firewatch_core::NodeInfo`] using credentials that cannot change it.

use std::time::Duration;

use firewatch_core::{Implementation, NodeInfo};
use serde::Deserialize;
use thiserror::Error;

pub mod cln;
pub mod lnd;
mod tls;

pub use tls::install_crypto_provider;

pub use cln::{ClnClient, ClnConfig};
pub use lnd::{LndClient, LndConfig};

/// Every message here can end up on the dashboard, so each one says what
/// happened in plain words and, where possible, what to check.
#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("config: {0}")]
    Config(String),
    #[error("Can't connect to {url}; is the node running?")]
    Unreachable { url: String },
    #[error("Timed out after {} s waiting for {url}", TIMEOUT.as_secs())]
    Timeout { url: String },
    #[error("TLS certificate not trusted; check tls_cert for {name}")]
    Tls { name: String },
    #[error("Credential rejected by the node ({0})")]
    CredentialRejected(String),
    #[error("request failed: {}", error_chain(.0))]
    Http(#[from] reqwest::Error),
    #[error("{method} refused (code {code}): {message}")]
    Rpc {
        method: String,
        code: i64,
        message: String,
    },
    #[error("Credential is not read-only: {0}")]
    NotReadOnly(String),
    #[error("unexpected response: {0}")]
    Unexpected(String),
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

/// How long one request to a node may take.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

/// Sorts a transport error into the plain cases an operator can act on.
pub(crate) fn transport_error(e: reqwest::Error, url: &str, name: &str) -> ConnectorError {
    let chain = error_chain(&e).to_ascii_lowercase();
    if e.is_timeout() {
        ConnectorError::Timeout { url: url.into() }
    } else if chain.contains("certificate") || chain.contains("tls") {
        ConnectorError::Tls { name: name.into() }
    } else if e.is_connect() {
        ConnectorError::Unreachable { url: url.into() }
    } else {
        ConnectorError::Http(e)
    }
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
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
