//! Private alerts over Nostr: NIP-17 gift-wrapped DMs.
//!
//! A "your node is vulnerable" message on a public channel tells attackers
//! which node to hit, so alerts are end-to-end encrypted to the operator's
//! key, and the gift wrap hides who sent them and when.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use firewatch_core::{Change, Finding, NodeReport, Status};
use futures::StreamExt;

pub mod lang;

pub use lang::{fill, Language, Strings, EN};
use nostr::nips::nip17::{self, PrivateDirectMessageBuilder};
use nostr_sdk::prelude::*;
use serde::Deserialize;
use thiserror::Error;

const TIMEOUT: Duration = Duration::from_secs(8);

/// How often a dropped relay is retried. nostr-sdk's default waits longer
/// after each failure, up to a minute, so after an outage the phone could
/// wait that long once the internet is back. A fixed, short retry costs one
/// failed connection every few seconds while offline.
const RECONNECT: Duration = Duration::from_secs(5);

/// How long an alert is kept for relays that didn't answer when it went out.
const RESEND_FOR: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Error)]
pub enum AlertError {
    #[error("nostr config: {0}")]
    Config(String),
    #[error("nostr: {0}")]
    Nostr(String),
}

fn nostr_err(e: impl std::fmt::Display) -> AlertError {
    AlertError::Nostr(e.to_string())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NostrConfig {
    /// The operator's public key: an npub, or the profile code (`nprofile`)
    /// from the phone app's share button, which also names the account's own
    /// relays. Alerts go only to this key.
    pub recipient: String,
    /// Firewatch's own secret key. Generated on first run if missing.
    pub secret_key_file: PathBuf,
    /// Relays to publish through. The recipient's NIP-17 inbox relays
    /// (kind 10050) are looked up and added automatically.
    pub relays: Vec<String>,
}

/// Checks `recipient` before it's written to a config, and returns it as it
/// should be written: the key or profile code, without a web link around it.
pub fn check_recipient(recipient: &str) -> Result<String, AlertError> {
    parse_recipient(recipient)?;
    Ok(unwrap_link(recipient).to_string())
}

/// The recipient's key, and the relays named with it in a profile code.
///
/// Takes an npub, a hex key, an `nprofile`, a `nostr:` link, or a web link
/// ending in one of these. Amethyst's share button gives a link such as
/// `https://njump.to/nprofile1…`, whose relays are the account's own. That's
/// where Amethyst keeps the list of relays the phone reads DMs from, and a
/// plain npub doesn't say where to look.
pub fn parse_recipient(recipient: &str) -> Result<(PublicKey, Vec<RelayUrl>), AlertError> {
    let code = unwrap_link(recipient);
    let bad = |e: String| AlertError::Config(format!("not a Nostr public key: {e}"));
    if code.starts_with("nprofile1") {
        let profile = Nip19Profile::from_bech32(code).map_err(|e| bad(e.to_string()))?;
        Ok((profile.public_key, profile.relays))
    } else {
        PublicKey::parse(code)
            .map(|key| (key, vec![]))
            .map_err(|e| bad(e.to_string()))
    }
}

/// The key or profile code inside a web or `nostr:` link.
fn unwrap_link(recipient: &str) -> &str {
    let s = recipient.trim();
    let s = if s.starts_with("https://") || s.starts_with("http://") {
        s.trim_end_matches('/').rsplit('/').next().unwrap_or(s)
    } else {
        s
    };
    s.strip_prefix("nostr:").unwrap_or(s)
}

pub struct NostrAlerter {
    client: Client,
    keys: Keys,
    recipient: PublicKey,
    /// The relays in the config, where the recipient's inbox list is looked up.
    configured: Vec<RelayUrl>,
    /// Relays named in the recipient's profile code. The inbox list is looked
    /// up there too; alerts aren't sent there.
    hints: Vec<RelayUrl>,
    /// Where alerts go: the configured relays, then the recipient's inbox.
    targets: Mutex<Vec<RelayUrl>>,
    /// Whether a relay has answered the inbox lookup. Until one has (no
    /// connection at start), it's tried again before each send.
    inbox_checked: AtomicBool,
    /// Alerts sent while some relays didn't answer, for those relays.
    skipped: Mutex<Vec<Skipped>>,
}

/// An alert that some relays didn't take because they didn't answer.
#[derive(Clone)]
struct Skipped {
    /// The signed event itself: sending the same one again can't show twice.
    event: Event,
    relays: Vec<RelayUrl>,
    sent_at: Instant,
}

impl NostrAlerter {
    pub async fn connect(config: &NostrConfig) -> Result<Self, AlertError> {
        let (recipient, hints) = parse_recipient(&config.recipient)
            .map_err(|e| AlertError::Config(format!("recipient: {e}")))?;
        let keys = load_or_create_keys(&config.secret_key_file)?;
        if config.relays.is_empty() {
            return Err(AlertError::Config(
                "relays must list at least one relay".into(),
            ));
        }

        let client = Client::builder()
            .authenticator(SignerAuthenticator::new(keys.clone()))
            .build();
        let mut configured = Vec::new();
        for relay in &config.relays {
            let url = RelayUrl::parse(relay)
                .map_err(|e| AlertError::Config(format!("relay {relay}: {e}")))?;
            add_relay(&client, &url).await?;
            configured.push(url);
        }
        let hints: Vec<RelayUrl> = hints
            .into_iter()
            .filter(|url| !configured.contains(url))
            .collect();
        for url in &hints {
            add_relay(&client, url).await?;
        }
        client.connect().and_wait(TIMEOUT).await;

        let alerter = NostrAlerter {
            client,
            keys,
            recipient,
            targets: Mutex::new(configured.clone()),
            configured,
            hints,
            inbox_checked: AtomicBool::new(false),
            skipped: Mutex::new(Vec::new()),
        };
        alerter.find_inbox().await;
        Ok(alerter)
    }

    /// Adds the relays where the recipient reads DMs (their NIP-17 inbox
    /// list, kind 10050), once a relay has answered. Returns the relays
    /// added; none if the lookup was done already, nobody answered yet, or
    /// the recipient has no list. A listed relay that can't be added is
    /// skipped, so it never stops an alert.
    ///
    /// nostr-sdk's own lookup answers "no events" both when a relay has no
    /// list and when it can't be reached, so only a relay that finishes its
    /// answer ([`ask`]) counts.
    pub async fn find_inbox(&self) -> Vec<RelayUrl> {
        if self.inbox_checked.load(Ordering::Relaxed) {
            return vec![];
        }
        let mut answered = false;
        let mut newest: Option<Event> = None;
        let lookup: Vec<RelayUrl> = self.configured.iter().chain(&self.hints).cloned().collect();
        for answer in self.ask_all(&lookup).await {
            let Some(Answer::Events(events)) = answer.1 else {
                continue;
            };
            answered = true;
            for event in events {
                if newest
                    .as_ref()
                    .is_none_or(|n| event.created_at > n.created_at)
                {
                    newest = Some(event);
                }
            }
        }
        if !answered {
            return vec![];
        }
        self.inbox_checked.store(true, Ordering::Relaxed);

        let mut added = Vec::new();
        if let Some(event) = newest {
            for url in nip17::extract_relay_list(&event) {
                if !self.relays().contains(&url) && add_relay(&self.client, &url).await.is_ok() {
                    added.push(url);
                }
            }
        }
        if !added.is_empty() {
            self.client.connect().and_wait(TIMEOUT).await;
            self.targets
                .lock()
                .expect("targets lock")
                .extend(added.iter().cloned());
        }
        added
    }

    /// Asks every relay in `urls` at once for the recipient's inbox list.
    async fn ask_all(&self, urls: &[RelayUrl]) -> Vec<(RelayUrl, Option<Answer>)> {
        let filter = Filter::new()
            .author(self.recipient)
            .kind(Kind::InboxRelays)
            .limit(1);
        let mut relays = Vec::new();
        for url in urls {
            if let Ok(Some(relay)) = self.client.relay(url).await {
                relays.push((url.clone(), relay));
            }
        }
        let answers =
            futures::future::join_all(relays.iter().map(|(_, r)| ask(r, filter.clone()))).await;
        relays
            .into_iter()
            .map(|(url, _)| url)
            .zip(answers)
            .collect()
    }

    /// Whether the recipient's inbox list has been looked up yet.
    pub fn inbox_checked(&self) -> bool {
        self.inbox_checked.load(Ordering::Relaxed)
    }

    /// Firewatch's own public key, so the operator can recognise the sender.
    pub fn sender(&self) -> String {
        self.keys
            .public_key()
            .to_bech32()
            .unwrap_or_else(|_| self.keys.public_key().to_hex())
    }

    pub fn relays(&self) -> Vec<RelayUrl> {
        self.targets.lock().expect("targets lock").clone()
    }

    /// Sends one encrypted DM. Succeeds if at least one relay accepted it.
    /// Looks up the recipient's inbox relays first if that hasn't worked yet.
    ///
    /// Only relays that answer right now ([`ask`]) get the message. When the
    /// connection drops, a relay still looks connected until its socket times
    /// out; nostr-sdk then queues the message, reports a timeout, and delivers
    /// the queued copy once the connection is back. The caller, told it
    /// failed, sends it again, and the phone gets it twice (seen on a phone).
    pub async fn send(&self, text: &str) -> Result<usize, AlertError> {
        self.find_inbox().await;
        let targets = self.relays();
        let connected: Vec<RelayUrl> = self
            .ask_all(&targets)
            .await
            .into_iter()
            .filter_map(|(url, answer)| answer.map(|_| url))
            .collect();
        if connected.is_empty() {
            return Err(AlertError::Nostr(
                "no relay answered (no internet, or the relays are down)".into(),
            ));
        }
        let event = PrivateDirectMessageBuilder::new(self.recipient, text)
            .finalize(&self.keys)
            .map_err(nostr_err)?;
        let output = self
            .client
            .send_event(&event)
            .to(connected.clone())
            .await
            .map_err(nostr_err)?;
        let silent: Vec<RelayUrl> = targets
            .into_iter()
            .filter(|url| !connected.contains(url))
            .collect();
        if !output.success.is_empty() && !silent.is_empty() {
            self.skipped.lock().expect("skipped lock").push(Skipped {
                event,
                relays: silent,
                sent_at: Instant::now(),
            });
        }
        if output.success.is_empty() {
            let reasons: Vec<String> = output
                .failed
                .iter()
                .map(|(url, why)| format!("{url}: {why}"))
                .collect();
            return Err(AlertError::Nostr(format!(
                "no relay accepted the alert ({})",
                reasons.join("; ")
            )));
        }
        Ok(output.success.len())
    }

    /// Sends earlier alerts to the relays that didn't answer when they went
    /// out and answer now: the same signed event, so the phone still shows
    /// it once. A relay that comes back after the alert went out may be the
    /// one the phone reads from. Each relay gets one try once it answers, for
    /// up to [`RESEND_FOR`]. Returns how many relays took an alert.
    pub async fn resend_skipped(&self) -> usize {
        let pending: Vec<Skipped> = {
            let mut skipped = self.skipped.lock().expect("skipped lock");
            skipped.retain(|s| s.sent_at.elapsed() < RESEND_FOR);
            skipped.clone()
        };
        let mut delivered = 0;
        let mut left = Vec::new();
        for mut item in pending.iter().cloned() {
            let answered: Vec<RelayUrl> = self
                .ask_all(&item.relays)
                .await
                .into_iter()
                .filter_map(|(url, answer)| answer.map(|_| url))
                .collect();
            if !answered.is_empty() {
                if let Ok(output) = self
                    .client
                    .send_event(&item.event)
                    .to(answered.clone())
                    .await
                {
                    delivered += output.success.len();
                }
                item.relays.retain(|url| !answered.contains(url));
            }
            if !item.relays.is_empty() {
                left.push(item);
            }
        }
        // Keep what was added meanwhile, replace what was just tried.
        let tried: Vec<EventId> = pending.iter().map(|s| s.event.id).collect();
        let mut skipped = self.skipped.lock().expect("skipped lock");
        skipped.retain(|s| !tried.contains(&s.event.id));
        skipped.extend(left);
        delivered
    }
}

/// How long a relay has to finish answering before it counts as unreachable.
const ANSWER: Duration = Duration::from_secs(4);

/// How a relay answered a request.
enum Answer {
    /// It sent these events, then said that was all (EOSE).
    Events(Vec<Event>),
    /// It refused the request (CLOSED), e.g. it wants a login first.
    Refused,
}

/// Sends one request to `relay` and waits for it to finish answering.
/// `None` if it doesn't within [`ANSWER`]: down, or a connection that died
/// without closing, which still shows as connected.
async fn ask(relay: &Relay, filter: Filter) -> Option<Answer> {
    if !relay.status().is_connected() {
        return None;
    }
    // Listening before asking, so the answer can't be missed.
    let mut notes = relay.notifications();
    let id = SubscriptionId::generate();
    relay
        .send_msg(ClientMessage::req(id.clone(), vec![filter]))
        .await
        .ok()?;
    let mut events = Vec::new();
    let answer = tokio::time::timeout(ANSWER, async {
        while let Some(note) = notes.next().await {
            match note {
                RelayNotification::Event {
                    subscription_id,
                    event,
                } if subscription_id == id => events.push(*event),
                RelayNotification::Message { message } => match *message {
                    RelayMessage::EndOfStoredEvents(sub) if *sub == id => return Some(true),
                    RelayMessage::Closed {
                        subscription_id, ..
                    } if *subscription_id == id => return Some(false),
                    _ => {}
                },
                _ => {}
            }
        }
        None
    })
    .await
    .ok()
    .flatten();
    let _ = relay.send_msg(ClientMessage::close(id)).await;
    match answer? {
        true => Some(Answer::Events(events)),
        false => Some(Answer::Refused),
    }
}

/// Adds a relay that is retried every [`RECONNECT`] when it drops.
async fn add_relay(client: &Client, url: &RelayUrl) -> Result<(), AlertError> {
    client
        .add_relay(url.clone())
        .retry_interval(RECONNECT)
        .adjust_retry_interval(false)
        .await
        .map(|_| ())
        .map_err(nostr_err)
}

fn load_or_create_keys(path: &Path) -> Result<Keys, AlertError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Keys::parse(text.trim())
            .map_err(|e| AlertError::Config(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let keys = Keys::generate();
            let nsec = keys.secret_key().to_bech32().map_err(nostr_err)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .and_then(|mut f| f.write_all(nsec.as_bytes()).map(|_| f))
                .map_err(|e| AlertError::Config(format!("writing {}: {e}", path.display())))?;
            file.flush().ok();
            Ok(keys)
        }
        Err(e) => Err(AlertError::Config(format!(
            "reading {}: {e}",
            path.display()
        ))),
    }
}

/// The DM text for one change, in English.
pub fn message(change: &Change) -> String {
    message_in(change, &EN)
}

/// The DM text for one change, in `lang`'s words. Plain text, so every
/// Nostr client shows it.
pub fn message_in(change: &Change, lang: &Strings) -> String {
    match change {
        Change::Raised { node, finding, .. } => raised(node, finding, lang),
        Change::Resolved { node, finding, .. } => format!(
            "✅ Firewatch: {}\n{}\n\n{}",
            fill(
                lang.no_longer_affected,
                &[("node", &node.name), ("ids", &finding.advisory_id)]
            ),
            finding.title,
            finding.reason
        ),
        Change::Unreachable { node } => format!(
            "⚠️ Firewatch: {}\n{}",
            fill(lang.cant_read, &[("node", &node.name)]),
            node.error.as_deref().unwrap_or("unknown error")
        ),
        Change::Reachable { node } => {
            format!(
                "Firewatch: {}",
                fill(lang.readable_again, &[("node", &node.name)])
            )
        }
    }
}

/// One DM for everything that changed on one node in one check, in English.
pub fn node_message(node: &NodeReport, changes: &[&Change]) -> String {
    node_message_in(node, changes, &EN)
}

/// One DM for everything that changed on one node in one check, so the
/// operator's phone buzzes once per node, not once per advisory. `changes`
/// are that node's `Raised` and `Resolved` changes, worst first.
pub fn node_message_in(node: &NodeReport, changes: &[&Change], lang: &Strings) -> String {
    let raised: Vec<&Finding> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Raised { finding, .. } => Some(*finding),
            _ => None,
        })
        .collect();
    let resolved: Vec<&Finding> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Resolved { finding, .. } => Some(*finding),
            _ => None,
        })
        .collect();

    let mut text = if raised.is_empty() {
        format!(
            "✅ Firewatch: {}",
            fill(
                lang.no_longer_affected,
                &[("node", &node.name), ("ids", &ids(&resolved))]
            )
        )
    } else {
        let affected = raised
            .iter()
            .filter(|f| f.status == Status::Affected)
            .count();
        let cant_tell = raised.len() - affected;
        let icon = if affected > 0 { "🔴" } else { "🟠" };
        let mut counts = Vec::new();
        if affected > 0 {
            counts.push(fill(lang.affected_count, &[("n", &affected.to_string())]));
        }
        if cant_tell > 0 {
            counts.push(fill(lang.cant_tell_count, &[("n", &cant_tell.to_string())]));
        }
        let mut text = format!("{icon} Firewatch: {}: {}", node.name, counts.join(", "));
        if let Some(shared) = shared_reason(&raised) {
            // e.g. an unofficial build: one reason covers every advisory.
            text.push_str(&format!("\n\n{}", shared.reason));
            for check in &shared.manual_checks {
                text.push_str(&format!("\n\n{}: {check}", lang.check_by_hand));
            }
            text.push_str(&format!("\n\n{}:", lang.advisories));
            for f in &raised {
                text.push_str(&format!("\n• {}: {}", f.advisory_id, f.title));
            }
        } else {
            for f in &raised {
                text.push_str("\n\n");
                text.push_str(&finding_details(f, lang));
            }
        }
        if !resolved.is_empty() {
            text.push_str(&format!(
                "\n\n✅ {}",
                fill(lang.also_no_longer, &[("ids", &ids(&resolved))])
            ));
        }
        text
    };
    if raised.is_empty() {
        for f in &resolved {
            text.push_str(&format!("\n\n{}: {}\n{}", f.advisory_id, f.title, f.reason));
        }
    }
    text
}

/// The finding whose reason and hand checks every raised finding shares, if
/// they all share one and none is red (a red finding has its own fix).
fn shared_reason<'a>(raised: &[&'a Finding]) -> Option<&'a Finding> {
    let first = *raised.first()?;
    let shared = raised.len() > 1
        && raised.iter().all(|f| {
            f.status != Status::Affected
                && f.reason == first.reason
                && f.manual_checks == first.manual_checks
        });
    shared.then_some(first)
}

fn ids(findings: &[&Finding]) -> String {
    findings
        .iter()
        .map(|f| f.advisory_id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The severity and "exploited" words for a finding, in `lang`.
fn flags(f: &Finding, lang: &Strings) -> String {
    let mut flags = vec![lang.severity[f.severity as usize].to_string()];
    if f.exploited_in_wild {
        flags.push(lang.exploited.to_string());
    }
    flags.join(", ")
}

fn label(f: &Finding, lang: &Strings) -> &'static str {
    if f.status == Status::Affected {
        lang.affected_label
    } else {
        lang.cant_tell_label
    }
}

/// One finding's block inside a node message. The title, reason and the
/// developers' words stay in English.
fn finding_details(f: &Finding, lang: &Strings) -> String {
    let mut text = format!(
        "{}: {} ({}): {}\n{}",
        label(f, lang),
        f.advisory_id,
        flags(f, lang),
        f.title,
        f.reason
    );
    if f.status == Status::Affected {
        text.push_str(&format!("\n{}: \"{}\"", lang.developers_say, f.steps));
    }
    for check in &f.manual_checks {
        text.push_str(&format!("\n{}: {check}", lang.check_by_hand));
    }
    text.push_str(&format!("\n{}: {}", lang.source, f.source));
    text
}

fn raised(node: &NodeReport, f: &Finding, lang: &Strings) -> String {
    let icon = if f.status == Status::Affected {
        "🔴"
    } else {
        "🟠"
    };
    let mut text = format!(
        "{icon} Firewatch: {}\n{} ({}): {}\n\n{}",
        fill(
            lang.node_status,
            &[("node", &node.name), ("label", label(f, lang))]
        ),
        f.advisory_id,
        flags(f, lang),
        f.title,
        f.reason
    );
    if f.status == Status::Affected {
        text.push_str(&format!("\n\n{}: \"{}\"", lang.developers_say, f.steps));
    }
    for check in &f.manual_checks {
        text.push_str(&format!("\n\n{}: {check}", lang.check_by_hand));
    }
    text.push_str(&format!("\n\n{}: {}", lang.source, f.source));
    text
}

#[cfg(test)]
mod tests {
    use firewatch_core::{Action, Implementation, Severity};

    use super::*;

    #[test]
    fn a_recipient_can_be_a_key_or_a_profile_code_with_relays() {
        let key = Keys::generate().public_key();
        let relays = vec![
            RelayUrl::parse("wss://nostr.mom").unwrap(),
            RelayUrl::parse("wss://nos.lol").unwrap(),
        ];
        let npub = key.to_bech32().unwrap();
        let nprofile = Nip19Profile::new(key, relays.clone()).to_bech32().unwrap();

        assert_eq!(parse_recipient(&npub).unwrap(), (key, vec![]));
        assert_eq!(parse_recipient(&key.to_hex()).unwrap(), (key, vec![]));
        for form in [
            nprofile.clone(),
            format!("nostr:{nprofile}"),
            format!("  https://njump.to/{nprofile}/ "),
        ] {
            assert_eq!(
                parse_recipient(&form).unwrap(),
                (key, relays.clone()),
                "{form}"
            );
        }
        // Written to the config without the link around it.
        assert_eq!(
            check_recipient(&format!("https://njump.to/{nprofile}")).unwrap(),
            nprofile
        );
        for bad in ["", "npub1nope", "https://njump.to/", "nsec1abc"] {
            assert!(parse_recipient(bad).is_err(), "{bad}");
        }
    }

    fn node() -> NodeReport {
        NodeReport {
            name: "old-cln".into(),
            implementation: Implementation::Cln,
            version: Some("v26.06.7".into()),
            network: Some("regtest".into()),
            status: Status::Affected,
            error: None,
            build_check: Default::default(),
            exposure: vec![],
            findings: vec![],
        }
    }

    fn finding(status: Status) -> Finding {
        Finding {
            advisory_id: "FW-2026-003".into(),
            title: "Vulnerabilities fixed in 26.06.8".into(),
            severity: Severity::Critical,
            exploited_in_wild: true,
            verified: true,
            fixed: Some("26.06.8".into()),
            source: "https://github.com/ElementsProject/lightning/releases/tag/v26.06.8".into(),
            status,
            reason: "Version 26.06.7 is in the affected range < 26.06.8.".into(),
            manual_checks: vec![],
            in_range: true,
            action: Action::Upgrade,
            steps: "We strongly recommend upgrading to this release.".into(),
        }
    }

    #[test]
    fn affected_message_quotes_fix_and_source() {
        let (n, f) = (node(), finding(Status::Affected));
        let text = message(&Change::Raised {
            node: &n,
            finding: &f,
            was: None,
        });
        assert!(text.starts_with("🔴 Firewatch: old-cln is AFFECTED"));
        assert!(text.contains("critical, exploited in the wild"));
        assert!(text.contains("Developers say: \"We strongly recommend upgrading"));
        assert!(text.ends_with(
            "Source: https://github.com/ElementsProject/lightning/releases/tag/v26.06.8"
        ));
    }

    #[test]
    fn resolved_message_says_no_longer_affected() {
        let (n, f) = (node(), finding(Status::NotAffected));
        let text = message(&Change::Resolved {
            node: &n,
            finding: &f,
            was: Status::Affected,
        });
        assert!(text.starts_with("✅ Firewatch: old-cln is no longer affected by FW-2026-003"));
    }

    #[test]
    fn creates_key_file_once_with_private_permissions() {
        let dir = std::env::temp_dir().join(format!("fw-keys-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("firewatch.nsec");
        let first = load_or_create_keys(&path).unwrap();
        let second = load_or_create_keys(&path).unwrap();
        assert_eq!(first.public_key(), second.public_key());
        let mode = std::os::unix::fs::PermissionsExt::mode(
            &std::fs::metadata(&path).unwrap().permissions(),
        );
        assert_eq!(mode & 0o777, 0o600);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn finding_with(id: &str, status: Status, reason: &str) -> Finding {
        Finding {
            advisory_id: id.into(),
            reason: reason.into(),
            ..finding(status)
        }
    }

    #[test]
    fn one_message_per_node_lists_every_finding() {
        let n = node();
        let a = finding_with("FW-2026-003", Status::Affected, "r1");
        let b = finding_with("FW-2026-006", Status::Affected, "r2");
        let changes = [
            Change::Raised {
                node: &n,
                finding: &a,
                was: None,
            },
            Change::Raised {
                node: &n,
                finding: &b,
                was: None,
            },
        ];
        let text = node_message(&n, &changes.iter().collect::<Vec<_>>());
        assert!(
            text.starts_with("🔴 Firewatch: old-cln: 2 affected"),
            "{text}"
        );
        assert!(text.contains("AFFECTED: FW-2026-003") && text.contains("AFFECTED: FW-2026-006"));
        assert_eq!(text.matches("Developers say:").count(), 2);
    }

    #[test]
    fn a_shared_reason_is_stated_once() {
        let n = node();
        let reason = "its build (fc472ef) is not the official release";
        let fs: Vec<Finding> = ["FW-2026-007", "FW-2026-008", "FW-2026-009"]
            .iter()
            .map(|id| Finding {
                manual_checks: vec!["Confirm where this binary came from.".into()],
                ..finding_with(id, Status::CantTell, reason)
            })
            .collect();
        let changes: Vec<Change> = fs
            .iter()
            .map(|f| Change::Raised {
                node: &n,
                finding: f,
                was: None,
            })
            .collect();
        let text = node_message(&n, &changes.iter().collect::<Vec<_>>());
        assert!(
            text.starts_with("🟠 Firewatch: old-cln: 3 can't tell"),
            "{text}"
        );
        assert_eq!(text.matches(reason).count(), 1, "{text}");
        assert_eq!(text.matches("Check by hand:").count(), 1);
        assert!(text.contains("• FW-2026-009"));
    }

    #[test]
    fn fixes_on_one_node_are_one_message() {
        let n = node();
        let a = finding_with("FW-2026-003", Status::NotAffected, "includes the fix");
        let b = finding_with("FW-2026-006", Status::NotAffected, "includes the fix");
        let changes = [
            Change::Resolved {
                node: &n,
                finding: &a,
                was: Status::Affected,
            },
            Change::Resolved {
                node: &n,
                finding: &b,
                was: Status::Affected,
            },
        ];
        let text = node_message(&n, &changes.iter().collect::<Vec<_>>());
        assert!(
            text.starts_with(
                "✅ Firewatch: old-cln is no longer affected by FW-2026-003, FW-2026-006"
            ),
            "{text}"
        );
    }
}
