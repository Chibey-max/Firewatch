//! The exposure check: does a node's API listen beyond this machine?
//!
//! A leaked rune or macaroon is only usable by someone who can reach the API.
//! In August 2026 macaroons stolen through BTCPay Server were used to take
//! control of LND nodes, and the advisory says: "Do not leave an affected LND
//! deployment accessible from the internet."
//!
//! This isn't an advisory verdict: a fully patched node can still be exposed.
//! So it is reported next to the findings and never changes the node's status.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::node::{Api, ApiListen, Listen};

/// The BTCPay Server 2.4.2 security advisory.
pub const SOURCE: &str = "https://blog.btcpayserver.org/security-advisory-btcpay-server-2-4-2/";
/// Quoted word for word from [`SOURCE`].
pub const QUOTE: &str = "Do not leave an affected LND deployment accessible from the internet.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExposureStatus {
    /// Listens only on this machine.
    Local,
    /// The address couldn't be read, or names a host Firewatch can't judge.
    Unknown,
    /// Listens on all interfaces, or on an address other machines can reach.
    Exposed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exposure {
    pub api: Api,
    pub status: ExposureStatus,
    /// What the node's config says, e.g. `["0.0.0.0:8080"]`.
    pub addresses: Vec<String>,
    /// One plain sentence.
    pub reason: String,
    /// What to check by hand; set unless the API is local.
    pub manual_check: Option<String>,
    /// [`SOURCE`], unless the API is local.
    pub source: Option<String>,
}

/// Where one listen address reaches.
#[derive(Debug, PartialEq, Eq)]
enum Reach {
    ThisMachine,
    AllInterfaces,
    /// A specific address other machines may reach, e.g. a LAN IP.
    Interface,
    /// A host name Firewatch can't resolve without guessing.
    Name,
}

fn reach(address: &str) -> Reach {
    if address.starts_with("unix:") {
        return Reach::ThisMachine;
    }
    let host = match address.strip_prefix('[') {
        Some(rest) => rest.split_once(']').map_or(rest, |(ip, _)| ip),
        None => address.rsplit_once(':').map_or(address, |(host, _)| host),
    };
    // Go and Core Lightning both read an empty host as "all interfaces".
    if host.is_empty() || host == "*" {
        return Reach::AllInterfaces;
    }
    if host.eq_ignore_ascii_case("localhost") {
        return Reach::ThisMachine;
    }
    match host.parse::<IpAddr>() {
        Ok(ip) if ip.is_loopback() => Reach::ThisMachine,
        Ok(ip) if ip.is_unspecified() => Reach::AllInterfaces,
        Ok(_) => Reach::Interface,
        Err(_) => Reach::Name,
    }
}

/// The port of `host:port` or `[v6]:port`; empty when there is none.
fn port(address: &str) -> &str {
    let after_host = match address.strip_prefix('[') {
        Some(rest) => rest.split_once(']').map_or("", |(_, after)| after),
        None => address,
    };
    after_host
        .rsplit_once(':')
        .map_or("", |(_, p)| p)
        .trim_start_matches(':')
}

fn label(api: Api) -> &'static str {
    match api {
        Api::Rest => "The REST API",
        Api::Grpc => "The gRPC API",
    }
}

/// [`label`] mid-sentence.
fn noun(api: Api) -> &'static str {
    match api {
        Api::Rest => "the REST API",
        Api::Grpc => "the gRPC API",
    }
}

/// Classifies one API from what its config says.
pub fn classify(api: &ApiListen) -> Exposure {
    let label = label(api.api);
    let addresses = match &api.listen {
        Listen::Unknown { why } => {
            return Exposure {
                api: api.api,
                status: ExposureStatus::Unknown,
                addresses: vec![],
                reason: format!("{label}'s listen address can't be read: {why}."),
                manual_check: Some(format!(
                    "Check where {} listens, and that it isn't reachable from the internet.",
                    noun(api.api)
                )),
                source: Some(SOURCE.into()),
            };
        }
        Listen::Read { addresses } => addresses,
    };
    let worst = addresses
        .iter()
        .map(|a| (a, reach(a)))
        .max_by_key(|(_, r)| match r {
            Reach::ThisMachine => 0,
            Reach::Name => 1,
            Reach::Interface => 2,
            Reach::AllInterfaces => 3,
        });
    let (reason, status, open) = match worst {
        None => {
            return Exposure {
                api: api.api,
                status: ExposureStatus::Unknown,
                addresses: vec![],
                reason: format!("{label} has no listen address in its config."),
                manual_check: Some(format!(
                    "Check where {} listens.",
                    noun(api.api)
                )),
                source: Some(SOURCE.into()),
            };
        }
        Some((_, Reach::ThisMachine)) => {
            return Exposure {
                api: api.api,
                status: ExposureStatus::Local,
                addresses: addresses.clone(),
                reason: format!("{label} only listens on this machine."),
                manual_check: None,
                source: None,
            };
        }
        Some((a, Reach::AllInterfaces)) => (
            format!("{label} listens on all interfaces ({a}), so a leaked credential works from anywhere that can reach it."),
            ExposureStatus::Exposed,
            a,
        ),
        Some((a, Reach::Interface)) => (
            format!("{label} listens on {a}, which other machines can reach, so a leaked credential works from there."),
            ExposureStatus::Exposed,
            a,
        ),
        Some((a, Reach::Name)) => (
            format!("{label} listens on {a}, a host name Firewatch can't judge."),
            ExposureStatus::Unknown,
            a,
        ),
    };
    // LND takes `restlisten=0.0.0.0` without a port, and uses its default.
    let port = match port(open) {
        "" => "this API's port".to_string(),
        p => format!("port {p}"),
    };
    Exposure {
        api: api.api,
        status,
        addresses: addresses.clone(),
        reason,
        manual_check: Some(format!(
            "Check that {port} isn't reachable from the internet: router port forwards, firewall, Tor or a reverse proxy."
        )),
        source: Some(SOURCE.into()),
    }
}

/// Every API of a node, most exposed first.
pub fn check(apis: &[ApiListen]) -> Vec<Exposure> {
    let mut out: Vec<Exposure> = apis.iter().map(classify).collect();
    out.sort_by(|a, b| b.status.cmp(&a.status).then(a.api.cmp(&b.api)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(api: Api, addresses: &[&str]) -> ApiListen {
        ApiListen {
            api,
            listen: Listen::Read {
                addresses: addresses.iter().map(|a| a.to_string()).collect(),
            },
        }
    }

    fn status(addresses: &[&str]) -> ExposureStatus {
        classify(&read(Api::Rest, addresses)).status
    }

    #[test]
    fn this_machine_only_is_local() {
        for a in [
            "127.0.0.1:8080",
            "127.0.0.1:3010",
            "[::1]:8080",
            "localhost:8080",
            "unix:///home/lnd/rest.sock",
        ] {
            assert_eq!(status(&[a]), ExposureStatus::Local, "{a}");
        }
        let e = classify(&read(Api::Rest, &["127.0.0.1:8080"]));
        assert!(e.manual_check.is_none() && e.source.is_none());
    }

    #[test]
    fn all_interfaces_is_exposed() {
        // `:8080` is all interfaces too, in Go (LND) and in Core Lightning.
        for a in ["0.0.0.0:8080", "[::]:8080", ":8080"] {
            assert_eq!(status(&[a]), ExposureStatus::Exposed, "{a}");
        }
        let e = classify(&read(Api::Rest, &["0.0.0.0:8080"]));
        assert!(e.reason.contains("all interfaces (0.0.0.0:8080)"));
        assert!(e
            .manual_check
            .unwrap()
            .starts_with("Check that port 8080 isn't reachable"));
        assert_eq!(e.source.as_deref(), Some(SOURCE));
    }

    #[test]
    fn an_address_without_a_port_still_gets_a_hint() {
        let e = classify(&read(Api::Rest, &["0.0.0.0"]));
        assert_eq!(e.status, ExposureStatus::Exposed);
        assert!(e
            .manual_check
            .unwrap()
            .starts_with("Check that this API's port isn't reachable"));
        assert_eq!(port("[::]"), "");
        assert_eq!(port("[::]:8080"), "8080");
    }

    #[test]
    fn a_lan_address_is_exposed() {
        assert_eq!(status(&["192.168.1.20:8080"]), ExposureStatus::Exposed);
        assert_eq!(status(&["[fe80::1]:8080"]), ExposureStatus::Exposed);
    }

    #[test]
    fn the_most_exposed_address_counts() {
        let e = classify(&read(Api::Grpc, &["127.0.0.1:10009", "0.0.0.0:10010"]));
        assert_eq!(e.status, ExposureStatus::Exposed);
        assert!(e.reason.contains("0.0.0.0:10010"));
        assert!(e.manual_check.unwrap().contains("port 10010"));
    }

    #[test]
    fn a_host_name_or_a_missing_address_is_unknown() {
        assert_eq!(status(&["mynode:8080"]), ExposureStatus::Unknown);
        assert_eq!(status(&[]), ExposureStatus::Unknown);
        let e = classify(&ApiListen {
            api: Api::Rest,
            listen: Listen::Unknown {
                why: "lnd_conf isn't set".into(),
            },
        });
        assert_eq!(e.status, ExposureStatus::Unknown);
        assert_eq!(
            e.reason,
            "The REST API's listen address can't be read: lnd_conf isn't set."
        );
        assert_eq!(
            e.manual_check.as_deref(),
            Some(
                "Check where the REST API listens, and that it isn't reachable from the internet."
            )
        );
    }

    #[test]
    fn exposed_apis_come_first() {
        let out = check(&[
            read(Api::Grpc, &["127.0.0.1:9736"]),
            read(Api::Rest, &["0.0.0.0:8080"]),
        ]);
        assert_eq!(out[0].status, ExposureStatus::Exposed);
        assert_eq!(out[1].status, ExposureStatus::Local);
    }
}
