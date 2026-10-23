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
    #[error("{url} doesn't answer HTTPS; {hint}")]
    NotTls { url: String, hint: &'static str },
    #[error("TLS with {url} failed ({reason}); check the url and port")]
    TlsFailed { url: String, reason: String },
    #[error("{url} expects HTTPS; change the url to https://")]
    NeedsHttps { url: String },
    #[error(
        "{url} answered{}, but not as a node's REST API; check the port (REST, not gRPC or p2p)",
        status.map(|s| format!(" with HTTP {s}")).unwrap_or_default()
    )]
    NotRestApi { url: String, status: Option<u16> },
    #[error("{url} closed the connection; check the port, or wait if the node is restarting")]
    ConnectionClosed { url: String },
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

/// The node's base url, checked up front: reqwest's own message for a url
/// without `http://` ("relative URL without a base") would reach the dashboard.
pub(crate) fn node_url(name: &str, url: &str) -> Result<String, ConnectorError> {
    match reqwest::Url::parse(url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") && u.host().is_some() => {
            Ok(url.trim_end_matches('/').to_string())
        }
        _ => Err(ConnectorError::Config(format!(
            "node '{name}': url \"{url}\" must be a full address starting with http:// or https://, e.g. https://127.0.0.1:8080"
        ))),
    }
}

/// What to check when an https:// url meets something that isn't TLS. LND's
/// REST port is always TLS; Core Lightning's clnrest may run over plain HTTP.
fn not_tls_hint(implementation: Implementation) -> &'static str {
    match implementation {
        Implementation::Cln => "check the port, or use http:// if clnrest runs over HTTP",
        _ => "check the port: LND's REST port, not its p2p or gRPC port",
    }
}

/// Sorts a transport error into the plain cases an operator can act on, by
/// the error types in its chain rather than by their text.
pub(crate) fn transport_error(
    e: reqwest::Error,
    url: &str,
    name: &str,
    implementation: Implementation,
) -> ConnectorError {
    let url = url.to_string();
    if e.is_timeout() {
        return ConnectorError::Timeout { url };
    }
    let causes = causes(&e);
    // Our pinned verifier and webpki both report a rejected certificate as
    // InvalidCertificate. InvalidMessage means the other end isn't speaking
    // TLS at all, e.g. an https:// url on a plain-HTTP port. Anything else is
    // a real TLS server we couldn't agree with, so say what rustls said.
    if let Some(tls) = find::<rustls::Error>(&causes) {
        return match tls {
            rustls::Error::InvalidCertificate(_) => ConnectorError::Tls { name: name.into() },
            rustls::Error::InvalidMessage(_) => ConnectorError::NotTls {
                url,
                hint: not_tls_hint(implementation),
            },
            other => ConnectorError::TlsFailed {
                url,
                reason: other.to_string(),
            },
        };
    }
    if let Some(http) = find::<hyper::Error>(&causes) {
        if http.is_parse() {
            // Something answered, but not in HTTP/1.1: gRPC, or another service.
            return ConnectorError::NotRestApi { url, status: None };
        }
        if http.is_incomplete_message() || http.is_closed() {
            return ConnectorError::ConnectionClosed { url };
        }
    }
    if e.is_connect() {
        // A TCP connect never ends in EOF; a TLS handshake the peer hangs up
        // on does (LND's p2p port, for one).
        let handshake_eof = causes.iter().any(|c| {
            c.downcast_ref::<std::io::Error>()
                .is_some_and(|io| io.kind() == std::io::ErrorKind::UnexpectedEof)
        });
        return if handshake_eof {
            ConnectorError::NotTls {
                url,
                hint: not_tls_hint(implementation),
            }
        } else {
            ConnectorError::Unreachable { url }
        };
    }
    ConnectorError::Http(e)
}

/// Reads a node's JSON reply. A body that isn't JSON means something other
/// than the node's REST API answered.
pub(crate) async fn read_json(
    response: reqwest::Response,
    url: &str,
    name: &str,
    implementation: Implementation,
) -> Result<serde_json::Value, ConnectorError> {
    let status = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| transport_error(e, url, name, implementation))?;
    serde_json::from_str(&text).map_err(|_| {
        // Go's net/http (LND) answers plain HTTP on a TLS port with this.
        if text.contains("HTTP request to an HTTPS server") {
            ConnectorError::NeedsHttps { url: url.into() }
        } else {
            // The status tells a proxy's 502 apart from a wrong port's 404.
            ConnectorError::NotRestApi {
                url: url.into(),
                status: Some(status),
            }
        }
    })
}

/// Every error in the chain. `io::Error::source()` skips the error it wraps,
/// so look inside each one with `get_ref()` too.
fn causes<'a>(
    e: &'a (dyn std::error::Error + 'static),
) -> Vec<&'a (dyn std::error::Error + 'static)> {
    let mut out = Vec::new();
    let mut next = Some(e);
    while let Some(err) = next {
        out.push(err);
        next = match err
            .downcast_ref::<std::io::Error>()
            .and_then(|io| io.get_ref())
        {
            Some(inner) => Some(inner as &(dyn std::error::Error + 'static)),
            None => err.source(),
        };
    }
    out
}

fn find<'a, T: std::error::Error + 'static>(
    causes: &[&'a (dyn std::error::Error + 'static)],
) -> Option<&'a T> {
    causes.iter().find_map(|c| c.downcast_ref::<T>())
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
