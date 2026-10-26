//! How Firewatch reads Core Lightning through runes of different reach (#17).
//! A fake clnrest answers the way Core Lightning 26.06.7 did to each rune,
//! and records every call, so the tests can check what Firewatch asks for.

use std::sync::{Arc, Mutex};

use firewatch_connectors::{fetch, http_client, ClnConfig, NodeConfig};
use firewatch_core::{Api, Listen};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Which rune the fake node was given.
#[derive(Clone, Copy)]
enum Rune {
    /// The one Firewatch recommends: four options by name.
    Narrow,
    /// The older `[["method=getinfo","method=listconfigs"]]`.
    Wide,
    /// `getinfo` only.
    GetinfoOnly,
}

const OPTIONS: [&str; 4] = ["clnrest-host", "clnrest-port", "grpc-host", "grpc-port"];

type Calls = Arc<Mutex<Vec<(String, Value)>>>;

fn refused() -> (u16, Value) {
    (401, json!({ "code": 1502, "message": "Not permitted" }))
}

fn option(name: &str) -> Option<Value> {
    match name {
        "clnrest-host" => Some(json!({ "value_str": "0.0.0.0" })),
        "clnrest-port" => Some(json!({ "value_int": 3010 })),
        "alias" => Some(json!({ "value_str": "fake" })),
        "bitcoin-rpcpassword" => Some(json!({ "value_str": "polarpass" })),
        // No cln-grpc on this node.
        _ => None,
    }
}

fn respond(rune: Rune, method: &str, params: &Value) -> (u16, Value) {
    let config = params.get("config").and_then(Value::as_str);
    match (method, rune) {
        ("getinfo", _) => (201, json!({ "version": "v26.06.7", "network": "regtest" })),
        ("listconfigs", Rune::GetinfoOnly) => refused(),
        ("listconfigs", Rune::Narrow) if !config.is_some_and(|c| OPTIONS.contains(&c)) => refused(),
        ("listconfigs", _) => match config {
            None => (
                201,
                json!({ "configs": {
                    "clnrest-host": option("clnrest-host"),
                    "clnrest-port": option("clnrest-port"),
                    "bitcoin-rpcpassword": option("bitcoin-rpcpassword"),
                }}),
            ),
            Some(name) => match option(name) {
                Some(entry) => (201, json!({ "configs": { name: entry } })),
                None => (
                    500,
                    json!({ "code": -32602, "message": "Unknown config option" }),
                ),
            },
        },
        _ => refused(),
    }
}

/// A fake clnrest for `rune`. Returns its URL and the calls it received.
async fn fake_node(rune: Rune) -> (String, Calls) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let calls: Calls = Arc::default();
    let log = calls.clone();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let log = log.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // Headers, then a body of Content-Length bytes.
                let (head, body) = loop {
                    let n = stream.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let head = String::from_utf8_lossy(&buf[..end]).to_string();
                    let length = head
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse::<usize>().ok())?
                        })
                        .unwrap_or(0);
                    if buf.len() >= end + 4 + length {
                        break (head, buf[end + 4..end + 4 + length].to_vec());
                    }
                };
                let method = head
                    .split_whitespace()
                    .nth(1)
                    .and_then(|path| path.rsplit('/').next())
                    .unwrap_or_default()
                    .to_string();
                let json_body = head
                    .to_ascii_lowercase()
                    .contains("content-type: application/json");
                // clnrest ignores a body that isn't sent as JSON.
                let params: Value = if json_body {
                    serde_json::from_slice(&body).unwrap_or(json!({}))
                } else {
                    json!({})
                };
                log.lock().unwrap().push((method.clone(), params.clone()));
                let (status, reply) = respond(rune, &method, &params);
                let reply = reply.to_string();
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    (url, calls)
}

async fn read(rune: Rune) -> (firewatch_core::NodeInfo, Vec<(String, Value)>) {
    let (url, calls) = fake_node(rune).await;
    let node = NodeConfig::Cln(ClnConfig {
        name: "cln".into(),
        url,
        rune: Some("test-rune".into()),
        rune_file: None,
        tls_cert: None,
    });
    let info = fetch(&node, &http_client())
        .await
        .expect("node is readable");
    let calls = calls.lock().unwrap().clone();
    (info, calls)
}

#[tokio::test]
async fn never_asks_for_every_option() {
    for rune in [Rune::Narrow, Rune::Wide, Rune::GetinfoOnly] {
        let (_, calls) = read(rune).await;
        for (method, params) in calls.iter().filter(|(m, _)| m == "listconfigs") {
            let asked = params["config"].as_str();
            assert!(
                asked.is_some_and(|c| OPTIONS.contains(&c) || c == "alias"),
                "{method} asked for {params}"
            );
        }
    }
}

#[tokio::test]
async fn the_narrow_rune_reads_the_apis_with_no_warning() {
    let (info, _) = read(Rune::Narrow).await;
    assert!(info.warnings.is_empty(), "{:?}", info.warnings);
    // No cln-grpc: its options are unknown to the node, so no gRPC entry.
    assert_eq!(info.apis.len(), 1);
    assert_eq!(info.apis[0].api, Api::Rest);
    assert_eq!(
        info.apis[0].listen,
        Listen::Read {
            addresses: vec!["0.0.0.0:3010".into()]
        }
    );
    assert!(info.config.is_none());
}

#[tokio::test]
async fn the_wide_rune_still_works_but_is_warned_about() {
    let (info, _) = read(Rune::Wide).await;
    assert_eq!(info.version, "v26.06.7");
    assert_eq!(info.warnings.len(), 1);
    let warning = &info.warnings[0];
    assert!(
        warning.contains("can read every config option"),
        "{warning}"
    );
    assert!(warning.contains("pnameconfig=clnrest-host"), "{warning}");
    assert!(
        !warning.contains("polarpass") && !warning.contains("fake"),
        "{warning}"
    );
    assert_eq!(
        info.apis[0].listen,
        Listen::Read {
            addresses: vec!["0.0.0.0:3010".into()]
        }
    );
}

#[tokio::test]
async fn a_rune_without_listconfigs_leaves_the_apis_unknown() {
    let (info, _) = read(Rune::GetinfoOnly).await;
    assert!(info.warnings.is_empty());
    assert_eq!(info.apis.len(), 2);
    for api in &info.apis {
        assert!(
            matches!(&api.listen, Listen::Unknown { why } if why.starts_with("the rune doesn't allow reading")),
            "{api:?}"
        );
    }
}
