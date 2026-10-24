//! What the operator reads when a node's url points at the wrong thing. Each
//! test stands up a local socket that misbehaves the way a real wrong port
//! did against Polar's LND and Core Lightning images (A2 error matrix).
//!
//! `fixtures/test-{a,b}.{cert,key}.pem` are throwaway self-signed certs for
//! the TLS server below; no node uses them.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use firewatch_connectors::{fetch, http_client, ClnConfig, LndConfig, NodeConfig};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const CERT_A: &[u8] = include_bytes!("fixtures/test-a.cert.pem");
const KEY_A: &[u8] = include_bytes!("fixtures/test-a.key.pem");
const CERT_B: &[u8] = include_bytes!("fixtures/test-b.cert.pem");

/// What a fake server does with one connection.
#[derive(Clone, Copy)]
enum Reply {
    /// Read the request, then send these bytes and close.
    Send(&'static [u8]),
    /// Read the first bytes, then hang up without a word.
    HangUp,
    /// Accept, then never read or answer.
    Silent,
}

async fn serve(reply: Reply) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(answer(stream, reply));
        }
    });
    addr.to_string()
}

async fn answer(mut stream: TcpStream, reply: Reply) {
    let mut buf = [0u8; 4096];
    match reply {
        Reply::Silent => {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
        Reply::HangUp => {
            let _ = stream.read(&mut buf).await;
        }
        Reply::Send(bytes) => {
            let _ = stream.read(&mut buf).await;
            let _ = stream.write_all(bytes).await;
        }
    }
}

/// A real TLS server presenting `cert_a`, answering every request with
/// `reply`. Uses rustls directly, so no extra test dependencies.
fn tls_serve(reply: &'static [u8]) -> String {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let config = Arc::new(
        rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from_pem_slice(CERT_A).unwrap()],
                PrivateKeyDer::from_pem_slice(KEY_A).unwrap(),
            )
            .unwrap(),
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let config = config.clone();
            std::thread::spawn(move || {
                let mut conn = rustls::ServerConnection::new(config).unwrap();
                let mut tls = rustls::Stream::new(&mut conn, &mut stream);
                let mut buf = [0u8; 4096];
                if tls.read(&mut buf).is_ok() {
                    let _ = tls.write_all(reply);
                    let _ = tls.flush();
                }
            });
        }
    });
    addr
}

async fn cln_error(url: String) -> String {
    let node = NodeConfig::Cln(ClnConfig {
        name: "n".into(),
        url,
        rune: Some("rune".into()),
        rune_file: None,
    });
    fetch(&node, &http_client()).await.unwrap_err().to_string()
}

/// `test` names a directory of its own: tests run in parallel, and a shared
/// one let one test read a cert another was rewriting.
async fn lnd_error(test: &str, url: String, cert: &[u8]) -> String {
    firewatch_connectors::install_crypto_provider();
    let dir = std::env::temp_dir().join(format!("firewatch-errors-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let node = NodeConfig::Lnd(LndConfig {
        name: "n".into(),
        url,
        macaroon_file: write(&dir, "readonly.macaroon", b"\x02\x01"),
        tls_cert: Some(write(&dir, "pinned.pem", cert)),
    });
    let message = fetch(&node, &http_client()).await.unwrap_err().to_string();
    let _ = std::fs::remove_dir_all(&dir);
    message
}

fn write(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn assert_plain(message: &str, starts_with: &str) {
    assert!(
        message.starts_with(starts_with),
        "expected {starts_with:?}, got {message:?}"
    );
    for raw in [
        "reqwest",
        "hyper",
        "SendRequest",
        "error sending request",
        "builder error",
    ] {
        assert!(!message.contains(raw), "raw transport text in {message:?}");
    }
}

#[tokio::test]
async fn nothing_listening_is_unreachable() {
    let addr = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().to_string()
    };
    let url = format!("http://{addr}");
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("Can't connect to {url}; is the node running?"),
    );
}

#[tokio::test]
async fn a_node_that_never_answers_times_out() {
    // Accepts the connection, then says nothing: the 10 s request timeout.
    let url = format!("http://{}", serve(Reply::Silent).await);
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("Timed out after 10 s waiting for {url}"),
    );
}

#[tokio::test]
async fn a_url_without_a_scheme_is_a_config_error() {
    for url in ["127.0.0.1:8080", "htp://127.0.0.1:8080", ""] {
        assert_plain(
            &cln_error(url.into()).await,
            &format!("config: node 'n': url \"{url}\" must be a full address starting with http:// or https://"),
        );
    }
}

#[tokio::test]
async fn a_service_that_isnt_http_is_not_a_rest_api() {
    // LND's gRPC port answers HTTP/1.1 with HTTP/2 frames: hyper can't parse it.
    let url = format!(
        "http://{}",
        serve(Reply::Send(b"SSH-2.0-not-http\r\n")).await
    );
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("{url} answered, but not as a node's REST API"),
    );
}

#[tokio::test]
async fn a_port_that_hangs_up_closed_the_connection() {
    // Core Lightning's p2p port drops a request that isn't a Noise handshake.
    let url = format!("http://{}", serve(Reply::HangUp).await);
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("{url} closed the connection"),
    );
}

#[tokio::test]
async fn an_http_reply_that_isnt_json_says_its_status() {
    // A reverse proxy in front of a stopped node: the status is the clue.
    let url = format!(
        "http://{}",
        serve(Reply::Send(
            b"HTTP/1.1 502 Bad Gateway\r\ncontent-length: 11\r\nconnection: close\r\n\r\nBad Gateway"
        ))
        .await
    );
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("{url} answered with HTTP 502, but not as a node's REST API"),
    );
}

#[tokio::test]
async fn plain_http_on_lnds_tls_port_needs_https() {
    // Go's net/http, word for word, as LND's REST port sent it.
    let body = b"HTTP/1.0 400 Bad Request\r\n\r\nClient sent an HTTP request to an HTTPS server.\n";
    let url = format!("http://{}", serve(Reply::Send(body)).await);
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("{url} expects HTTPS; change the url to https://"),
    );
}

#[tokio::test]
async fn https_on_a_plain_http_port_gives_each_implementation_its_own_hint() {
    let url = format!(
        "https://{}",
        serve(Reply::Send(b"HTTP/1.1 400 Bad Request\r\n\r\n")).await
    );
    assert_plain(
        &lnd_error("plain-http", url.clone(), CERT_A).await,
        &format!("{url} doesn't answer HTTPS; check the port: LND's REST port"),
    );
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("{url} doesn't answer HTTPS; check the port, or use http://"),
    );
}

#[tokio::test]
async fn a_tls_handshake_hang_up_doesnt_answer_https() {
    // LND's p2p port reads the ClientHello and hangs up.
    let url = format!("https://{}", serve(Reply::HangUp).await);
    assert_plain(
        &lnd_error("hang-up", url.clone(), CERT_A).await,
        &format!("{url} doesn't answer HTTPS"),
    );
}

#[tokio::test]
async fn another_nodes_certificate_is_not_trusted() {
    // The server presents cert A; the config pins cert B.
    let url = format!("https://{}", tls_serve(b"HTTP/1.1 200 OK\r\n\r\n"));
    assert_plain(
        &lnd_error("wrong-cert", url, CERT_B).await,
        "TLS certificate not trusted; check tls_cert for n",
    );
}

#[tokio::test]
async fn the_pinned_certificate_gets_through_tls() {
    // Control for the test above: with the right pin, TLS succeeds and the
    // error comes from the HTTP reply instead.
    let url = format!(
        "https://{}",
        tls_serve(b"HTTP/1.1 404 Not Found\r\ncontent-length: 2\r\nconnection: close\r\n\r\nno")
    );
    assert_plain(
        &lnd_error("right-cert", url.clone(), CERT_A).await,
        &format!("{url} answered with HTTP 404, but not as a node's REST API"),
    );
}
