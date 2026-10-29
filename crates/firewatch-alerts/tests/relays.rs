//! Delivery across a lost connection, against relays running in this process.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

/// Forwards `from` to `to` until paused, then holds every connection open
/// and silent: what a relay looks like when the Wi-Fi drops. Nothing is
/// closed, so the client still thinks it's connected.
async fn pausable_proxy(from: u16, to: u16) -> Arc<AtomicBool> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let paused = Arc::new(AtomicBool::new(false));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", from))
        .await
        .unwrap();
    let flag = paused.clone();
    tokio::spawn(async move {
        while let Ok((client, _)) = listener.accept().await {
            let Ok(relay) = tokio::net::TcpStream::connect(("127.0.0.1", to)).await else {
                continue;
            };
            let (cr, cw) = client.into_split();
            let (rr, rw) = relay.into_split();
            for (mut r, mut w, flag) in [
                (
                    Box::new(cr) as Box<dyn tokio::io::AsyncRead + Unpin + Send>,
                    Box::new(rw) as Box<dyn tokio::io::AsyncWrite + Unpin + Send>,
                    flag.clone(),
                ),
                (Box::new(rr), Box::new(cw), flag.clone()),
            ] {
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 64 * 1024];
                    loop {
                        while flag.load(Ordering::SeqCst) {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        let n = match r.read(&mut buf).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => n,
                        };
                        // Read while paused: hold it until resumed, as a
                        // network that drops packets and later retransmits.
                        while flag.load(Ordering::SeqCst) {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        if w.write_all(&buf[..n]).await.is_err() {
                            return;
                        }
                    }
                });
            }
        }
    });
    paused
}

#[tokio::test]
async fn a_silent_drop_does_not_deliver_the_alert_twice() {
    let recipient = Keys::generate();
    let (relay_port, proxy_port) = (free_port(), free_port());
    let _relay = relay_on(relay_port).await;
    let paused = pausable_proxy(proxy_port, relay_port).await;
    let alerter = NostrAlerter::connect(&config("silent", &recipient, &[proxy_port]))
        .await
        .unwrap();
    alerter.send("before").await.unwrap();

    // The Wi-Fi drops: the connection stays open but goes quiet.
    paused.store(true, Ordering::SeqCst);
    assert!(alerter.send("failed try").await.is_err());

    // Back: only the retry may arrive.
    paused.store(false, Ordering::SeqCst);
    send_until_delivered(&alerter, "retry").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let mut got = messages(&recipient, relay_port).await;
    got.sort();
    assert_eq!(got, ["before", "retry"]);
}

/// The recipient as a profile code (`nprofile`) naming `relays`, the way
/// Amethyst's share button gives it.
fn profile_code(recipient: &Keys, relays: &[u16]) -> String {
    Nip19Profile::new(
        recipient.public_key(),
        relays.iter().map(|p| RelayUrl::parse(&url(*p)).unwrap()),
    )
    .to_bech32()
    .unwrap()
}

#[tokio::test]
async fn a_profile_code_finds_the_phones_relays() {
    // Amethyst keeps the DM inbox list only on the account's own relays,
    // which Firewatch isn't configured with.
    let recipient = Keys::generate();
    let (configured, own, inbox) = (free_port(), free_port(), free_port());
    let _configured = relay_on(configured).await;
    let _own = relay_on(own).await;
    let _inbox = relay_on(inbox).await;
    publish_inbox(&recipient, own, inbox).await;

    let mut config = config("profile", &recipient, &[configured]);
    config.recipient = format!("https://njump.to/{}", profile_code(&recipient, &[own]));
    let alerter = NostrAlerter::connect(&config).await.unwrap();
    alerter.send("found you").await.unwrap();
    assert_eq!(messages(&recipient, inbox).await, ["found you"]);
}

#[tokio::test]
async fn a_relay_skipped_during_a_send_gets_the_same_alert_later() {
    let recipient = Keys::generate();
    let (a, b, b_proxy) = (free_port(), free_port(), free_port());
    let _a = relay_on(a).await;
    let _b = relay_on(b).await;
    let paused = pausable_proxy(b_proxy, b).await;
    let alerter = NostrAlerter::connect(&config("skipped", &recipient, &[a, b_proxy]))
        .await
        .unwrap();

    // b is silent while the alert goes out, so only a takes it.
    paused.store(true, Ordering::SeqCst);
    assert_eq!(alerter.send("late relay").await.unwrap(), 1);
    assert!(messages(&recipient, b).await.is_empty());

    // b is back: the same alert reaches it, so the phone still sees one.
    paused.store(false, Ordering::SeqCst);
    let start = Instant::now();
    while alerter.resend_skipped().await == 0 {
        assert!(start.elapsed() < Duration::from_secs(30), "never re-sent");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert_eq!(messages(&recipient, b).await, ["late relay"]);
    assert_eq!(
        gift_wrap_ids(&recipient, a).await,
        gift_wrap_ids(&recipient, b).await,
        "a different event would show twice"
    );
    // Nothing left to re-send.
    assert_eq!(alerter.resend_skipped().await, 0);
}

/// The ids of the gift wraps a relay holds for the recipient.
async fn gift_wrap_ids(recipient: &Keys, port: u16) -> Vec<EventId> {
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
    wraps.iter().map(|e| e.id).collect()
}
