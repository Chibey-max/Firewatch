//! LND over its REST API, authenticated with `readonly.macaroon`.
//!
//! Firewatch only reads `GetInfo`, and refuses a macaroon that can write.
//! Most `lnd.conf` options are not exposed over RPC, so the node's config is
//! reported as unknown and config conditions come back "Can't tell".

use std::io::Read;
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
    /// Allow plain `http://` to another machine (LND with `no-rest-tls`).
    /// Off by default: the macaroon would cross the network unencrypted.
    #[serde(default)]
    pub allow_plain_http: bool,
}

pub struct LndClient {
    http: reqwest::Client,
    name: String,
    url: String,
    macaroon: String,
    lnd_conf: Option<PathBuf>,
    /// Set when `allow_plain_http` let a remote `http://` url through.
    plain_http: Option<String>,
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
        let url = crate::node_url(&config.name, &config.url)?;
        let plain_http = crate::plain_http_check(
            &config.name,
            &url,
            Implementation::Lnd,
            config.allow_plain_http,
        )?;
        Ok(LndClient {
            http: http
                .build()
                .map_err(|e| ConnectorError::Config(e.to_string()))?,
            name: config.name.clone(),
            url,
            macaroon,
            lnd_conf: config.lnd_conf.clone(),
            plain_http,
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
        node.warnings.extend(self.plain_http.iter().cloned());
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
    // Capped, so a wrong path (a log, a device) can't stall the check.
    let mut text = String::new();
    let read =
        std::fs::File::open(path).and_then(|f| f.take(MAX_LND_CONF + 1).read_to_string(&mut text));
    match read {
        Ok(n) if n as u64 > MAX_LND_CONF => unknown(format!(
            "{} is larger than {} KiB, so it isn't lnd.conf",
            path.display(),
            MAX_LND_CONF / 1024
        )),
        Ok(_) => parse_lnd_conf(&text),
        Err(e) => unknown(format!("reading {}: {e}", path.display())),
    }
}

/// lnd.conf is a few KiB; anything past this isn't one.
const MAX_LND_CONF: u64 = 1024 * 1024;
/// LND's defaults (`defaultRESTPort`, `defaultRPCPort` in its config.go).
const DEFAULT_REST_PORT: u16 = 8080;
const DEFAULT_RPC_PORT: u16 = 10009;

/// `restlisten` and `rpclisten` from lnd.conf, read the way LND reads them.
/// The file format is go-flags' INI: a line starting with `;` or `#` is a
/// comment, and a value starting with `"` is a quoted string. Each option may
/// appear more than once.
fn parse_lnd_conf(text: &str) -> Vec<ApiListen> {
    let values = |key: &str, default_port: u16| -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.starts_with(';') && !line.starts_with('#'))
            .filter_map(|line| line.split_once('='))
            .filter(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            .map(|(_, v)| unquote(v.trim()))
            .filter(|v| !v.is_empty())
            .map(|v| lnd_listen_address(&v, default_port))
            .collect()
    };
    [
        (Api::Rest, "restlisten", DEFAULT_REST_PORT),
        (Api::Grpc, "rpclisten", DEFAULT_RPC_PORT),
    ]
    .map(|(api, key, default_port)| {
        let addresses = values(key, default_port);
        let listen = if addresses.is_empty() {
            Listen::Unknown {
                why: format!("lnd.conf doesn't set {key}, and LND's command line can't be read"),
            }
        } else {
            Listen::Read { addresses }
        };
        ApiListen { api, listen }
    })
    .to_vec()
}

/// A go-flags INI value: `"0.0.0.0:8080"` gives `0.0.0.0:8080`.
fn unquote(value: &str) -> String {
    match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => value.to_string(),
    }
}

/// One listen address as LND itself reads it (`lncfg.ParseAddressString` and
/// `verifyPort`), as `host:port` or `unix://path`:
/// - an optional network: `tcp://`, `tcp4:` ... or `unix://`, `unix:`
/// - a bare port is `localhost:port` (LND's legacy shortcut)
/// - no port means the default one; `:` alone means all interfaces on it
fn lnd_listen_address(raw: &str, default_port: u16) -> String {
    let network_and_address = raw.split_once("://").or_else(|| raw.split_once(':'));
    let address = match network_and_address {
        // Exact names, as LND matches them: a host called `unixbox` is a host.
        Some(("unix" | "unixpacket", address)) => return format!("unix://{address}"),
        Some(("tcp" | "tcp4" | "tcp6", address)) => address,
        _ => raw,
    };
    match split_host_port(address) {
        Some(("", "")) => format!(":{default_port}"),
        Some(_) => address.to_string(),
        None if !address.is_empty() && address.bytes().all(|b| b.is_ascii_digit()) => {
            format!("localhost:{address}")
        }
        None if address.starts_with('[') => format!("{address}:{default_port}"),
        // An IPv6 address without brackets, like Go's JoinHostPort adds.
        None if address.contains(':') => format!("[{address}]:{default_port}"),
        None => format!("{address}:{default_port}"),
    }
}

/// Go's `net.SplitHostPort`: `[v6]:port` or `host:port` with one colon.
fn split_host_port(address: &str) -> Option<(&str, &str)> {
    if let Some(rest) = address.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        return Some((host, after.strip_prefix(':')?));
    }
    let (host, port) = address.split_once(':')?;
    (!port.contains(':')).then_some((host, port))
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
        warnings: vec![],
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
    fn listen_addresses_are_read_the_way_lnd_reads_them() {
        // lncfg.ParseAddressString and verifyPort (LND v0.20.0-beta).
        for (raw, read) in [
            ("0.0.0.0:8080", "0.0.0.0:8080"),
            ("tcp://0.0.0.0:8080", "0.0.0.0:8080"),
            ("tcp:0.0.0.0:8080", "0.0.0.0:8080"),
            ("tcp4://127.0.0.1:8080", "127.0.0.1:8080"),
            ("tcp6://[::]:8080", "[::]:8080"),
            ("8080", "localhost:8080"),
            ("0.0.0.0", "0.0.0.0:8080"),
            ("localhost", "localhost:8080"),
            ("[::]", "[::]:8080"),
            ("::", "[::]:8080"),
            (":", ":8080"),
            (":8080", ":8080"),
            ("unix:///run/lnd/rest.sock", "unix:///run/lnd/rest.sock"),
            ("unix:/run/lnd/rest.sock", "unix:///run/lnd/rest.sock"),
            ("unixpacket:/run/lnd/rest.sock", "unix:///run/lnd/rest.sock"),
            ("unixbox:8080", "unixbox:8080"),
        ] {
            assert_eq!(lnd_listen_address(raw, DEFAULT_REST_PORT), read, "{raw}");
        }
    }

    #[test]
    fn quoted_and_network_prefixed_values_are_read() {
        let apis = parse_lnd_conf(
            "[Application Options]\nrestlisten=\"0.0.0.0:8080\"\nrpclisten=tcp://0.0.0.0\n",
        );
        assert_eq!(
            addresses(&apis, Api::Rest),
            Listen::Read {
                addresses: vec!["0.0.0.0:8080".into()]
            }
        );
        assert_eq!(
            addresses(&apis, Api::Grpc),
            Listen::Read {
                addresses: vec!["0.0.0.0:10009".into()]
            }
        );
    }

    #[test]
    fn a_file_too_large_to_be_lnd_conf_is_unknown() {
        let path = std::env::temp_dir().join(format!("firewatch-big-{}.conf", std::process::id()));
        std::fs::write(&path, vec![b'#'; (MAX_LND_CONF + 1) as usize]).unwrap();
        let apis = listen_from_conf(Some(&path));
        let _ = std::fs::remove_file(&path);
        assert!(matches!(
            &apis[0].listen,
            Listen::Unknown { why } if why.contains("larger than 1024 KiB")
        ));
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
