//! What the operator reads when a node's url points at the wrong thing. Each
//! test stands up a local socket that misbehaves the way a real wrong port
//! did against Polar's LND and Core Lightning images (A2 error matrix).

use std::path::PathBuf;

use firewatch_connectors::{fetch, http_client, ClnConfig, LndConfig, NodeConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// What a fake server does with one connection.
#[derive(Clone, Copy)]
enum Reply {
    /// Read the request, then send these bytes and close.
    Send(&'static [u8]),
    /// Read the first bytes, then hang up without a word.
    HangUp,
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
    let _ = stream.read(&mut buf).await;
    if let Reply::Send(bytes) = reply {
        let _ = stream.write_all(bytes).await;
    }
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

async fn lnd_error(url: String) -> String {
    firewatch_connectors::install_crypto_provider();
    let dir = std::env::temp_dir().join(format!("firewatch-errors-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let macaroon = write(&dir, "readonly.macaroon", b"\x02\x01");
    // Never compared: every test here fails before the certificate is seen.
    let cert = write(
        &dir,
        "pinned.pem",
        b"-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n",
    );
    let node = NodeConfig::Lnd(LndConfig {
        name: "n".into(),
        url,
        macaroon_file: macaroon,
        tls_cert: Some(cert),
    });
    fetch(&node, &http_client()).await.unwrap_err().to_string()
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
    for raw in ["reqwest", "hyper", "SendRequest", "error sending request"] {
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
async fn an_http_reply_that_isnt_json_is_not_a_rest_api() {
    let url = format!(
        "http://{}",
        serve(Reply::Send(
            b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\nconnection: close\r\n\r\nhello"
        ))
        .await
    );
    assert_plain(
        &cln_error(url.clone()).await,
        &format!("{url} answered, but not as a node's REST API"),
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
async fn https_on_a_plain_http_port_doesnt_answer_https() {
    let url = format!(
        "https://{}",
        serve(Reply::Send(b"HTTP/1.1 400 Bad Request\r\n\r\n")).await
    );
    assert_plain(
        &lnd_error(url.clone()).await,
        &format!("{url} doesn't answer HTTPS"),
    );
}

#[tokio::test]
async fn a_tls_handshake_hang_up_doesnt_answer_https() {
    // LND's p2p port reads the ClientHello and hangs up.
    let url = format!("https://{}", serve(Reply::HangUp).await);
    assert_plain(
        &lnd_error(url.clone()).await,
        &format!("{url} doesn't answer HTTPS"),
    );
}
