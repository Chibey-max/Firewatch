//! Delivery across a lost connection, against relays running in this process.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use firewatch_alerts::{NostrAlerter, NostrConfig};
use nostr::nips::nip59::UnwrappedGift;
use nostr_sdk::prelude::*;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn url(port: u16) -> String {
    format!("ws://127.0.0.1:{port}")
}

async fn relay_on(port: u16) -> LocalRelay {
    let relay = LocalRelay::builder().port(port).build();
    relay.run().await.unwrap();
    relay
}

fn config(test: &str, recipient: &Keys, relays: &[u16]) -> NostrConfig {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("fw-relays-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    NostrConfig {
        recipient: recipient.public_key().to_hex(),
        secret_key_file: dir.join("firewatch.nsec"),
        relays: relays.iter().map(|p| url(*p)).collect(),
    }
}

/// The recipient says, as a phone app does, where it reads DMs.
async fn publish_inbox(recipient: &Keys, on: u16, inbox: u16) {
    let client = Client::default();
    client.add_relay(url(on)).await.unwrap();
    client.connect().and_wait(Duration::from_secs(3)).await;
    let event = EventBuilder::new(Kind::InboxRelays, "")
        .tag(Tag::parse(["relay", &url(inbox)]).unwrap())
        .finalize(recipient)
        .unwrap();
    client.send_event(&event).await.unwrap();
}

/// What the recipient's phone would show from this relay.
async fn messages(recipient: &Keys, port: u16) -> Vec<String> {
    let client = Client::default();
    client.add_relay(url(port)).await.unwrap();
    client.connect().and_wait(Duration::from_secs(3)).await;
    let wraps = client
        .fetch_events(
            Filter::new()
                .kind(Kind::GiftWrap)
                .pubkey(recipient.public_key()),
        )
        .timeout(Duration::from_secs(3))
        .await
        .unwrap();
    wraps
        .iter()
        .filter_map(|w| UnwrappedGift::from_gift_wrap(recipient, w).ok())
        .map(|g| g.rumor.content)
        .collect()
}

/// Sends until a relay takes it, as `watch` retries each check.
async fn send_until_delivered(alerter: &NostrAlerter, text: &str) {
    let start = Instant::now();
    while alerter.send(text).await.is_err() {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "not delivered after the relays came back"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::test]
async fn started_offline_it_still_finds_where_the_phone_reads() {
    let recipient = Keys::generate();
    let (configured, inbox) = (free_port(), free_port());
    // No connection when Firewatch starts, as after a power cut.
    let alerter = NostrAlerter::connect(&config("offline", &recipient, &[configured]))
        .await
        .unwrap();
    assert!(!alerter.inbox_checked());
    assert!(alerter.send("while offline").await.is_err());

    // The connection comes back.
    let _configured = relay_on(configured).await;
    let _inbox = relay_on(inbox).await;
    publish_inbox(&recipient, configured, inbox).await;

    send_until_delivered(&alerter, "back online").await;
    assert!(alerter.inbox_checked());
    assert!(alerter
        .relays()
        .contains(&RelayUrl::parse(&url(inbox)).unwrap()));
    assert_eq!(messages(&recipient, inbox).await, ["back online"]);
}

#[tokio::test]
async fn a_send_that_failed_is_never_delivered_later() {
    let recipient = Keys::generate();
    let port = free_port();
    let relay = relay_on(port).await;
    let alerter = NostrAlerter::connect(&config("dup", &recipient, &[port]))
        .await
        .unwrap();
    alerter.send("before").await.unwrap();

    // The connection drops; this try fails, so `watch` will send it again.
    relay.shutdown();
    drop(relay);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(alerter.send("failed try").await.is_err());

    // Back (an empty relay on the same port): only the retry may arrive.
    let _relay = relay_on(port).await;
    send_until_delivered(&alerter, "retry").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(messages(&recipient, port).await, ["retry"]);
}
