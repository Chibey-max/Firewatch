//! Prints the NIP-17 DMs sent to a key: a terminal stand-in for a phone.
//!
//!   cargo run -p firewatch-alerts --example inbox -- <nsec> [relay ...]
//!   cargo run -p firewatch-alerts --example inbox -- --new   # make a test key

use std::time::Duration;

use nostr::nips::nip59::UnwrappedGift;
use nostr_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(String::as_str) == Some("--new") {
        let keys = Keys::generate();
        println!("nsec: {}", keys.secret_key().to_bech32()?);
        println!("npub: {}", keys.public_key().to_bech32()?);
        return Ok(());
    }
    let keys = Keys::parse(&args.next().ok_or("usage: inbox <nsec> [relay ...]")?)?;
    let mut relays: Vec<String> = args.collect();
    if relays.is_empty() {
        relays = vec!["wss://relay.damus.io".into(), "wss://nos.lol".into()];
    }

    let client = Client::builder()
        .authenticator(SignerAuthenticator::new(keys.clone()))
        .build();
    for relay in &relays {
        client.add_relay(relay.as_str()).await?;
    }
    client.connect().and_wait(Duration::from_secs(8)).await;

    let filter = Filter::new().kind(Kind::GiftWrap).pubkey(keys.public_key());
    let events = client
        .fetch_events(filter)
        .timeout(Duration::from_secs(10))
        .await?;
    let mut messages: Vec<_> = events
        .iter()
        .filter_map(|e| UnwrappedGift::from_gift_wrap(&keys, e).ok())
        .filter(|g| g.rumor.kind == Kind::PrivateDirectMessage)
        .map(|g| (g.rumor.created_at, g.sender, g.rumor.content))
        .collect();
    messages.sort_by_key(|(at, _, _)| *at);
    for (at, sender, content) in messages {
        println!(
            "--- {} from {}\n{content}\n",
            at.as_secs(),
            sender.to_bech32()?
        );
    }
    Ok(())
}
