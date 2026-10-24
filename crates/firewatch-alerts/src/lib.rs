//! Private alerts over Nostr: NIP-17 gift-wrapped DMs.
//!
//! A "your node is vulnerable" message on a public channel tells attackers
//! which node to hit, so alerts are end-to-end encrypted to the operator's
//! key, and the gift wrap hides who sent them and when.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use firewatch_core::{Change, Finding, NodeReport, Status};
use nostr::nips::nip17::{self, PrivateDirectMessageBuilder};
use nostr_sdk::prelude::*;
use serde::Deserialize;
use thiserror::Error;

const TIMEOUT: Duration = Duration::from_secs(8);

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
    /// The operator's public key (npub or hex). Alerts go only to this key.
    pub recipient: String,
    /// Firewatch's own secret key. Generated on first run if missing.
    pub secret_key_file: PathBuf,
    /// Relays to publish through. The recipient's NIP-17 inbox relays
    /// (kind 10050) are looked up and added automatically.
    pub relays: Vec<String>,
}

pub struct NostrAlerter {
    client: Client,
    keys: Keys,
    recipient: PublicKey,
    targets: Vec<RelayUrl>,
}

impl NostrAlerter {
    pub async fn connect(config: &NostrConfig) -> Result<Self, AlertError> {
        let recipient = PublicKey::parse(&config.recipient)
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
        let mut targets = Vec::new();
        for relay in &config.relays {
            let url = RelayUrl::parse(relay)
                .map_err(|e| AlertError::Config(format!("relay {relay}: {e}")))?;
            client.add_relay(url.clone()).await.map_err(nostr_err)?;
            targets.push(url);
        }
        client.connect().and_wait(TIMEOUT).await;

        // Deliver where the recipient reads DMs, if they published a list.
        let inbox = Filter::new()
            .author(recipient)
            .kind(Kind::InboxRelays)
            .limit(1);
        if let Ok(events) = client.fetch_events(inbox).timeout(TIMEOUT).await {
            if let Some(event) = events.iter().max_by_key(|e| e.created_at) {
                for url in nip17::extract_relay_list(event) {
                    if !targets.contains(&url) {
                        client.add_relay(url.clone()).await.map_err(nostr_err)?;
                        targets.push(url);
                    }
                }
                client.connect().and_wait(TIMEOUT).await;
            }
        }

        Ok(NostrAlerter {
            client,
            keys,
            recipient,
            targets,
        })
    }

    /// Firewatch's own public key, so the operator can recognise the sender.
    pub fn sender(&self) -> String {
        self.keys
            .public_key()
            .to_bech32()
            .unwrap_or_else(|_| self.keys.public_key().to_hex())
    }

    pub fn relays(&self) -> &[RelayUrl] {
        &self.targets
    }

    /// Sends one encrypted DM. Succeeds if at least one relay accepted it.
    pub async fn send(&self, text: &str) -> Result<usize, AlertError> {
        let event = PrivateDirectMessageBuilder::new(self.recipient, text)
            .finalize(&self.keys)
            .map_err(nostr_err)?;
        let output = self
            .client
            .send_event(&event)
            .to(self.targets.iter().cloned())
            .await
            .map_err(nostr_err)?;
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

/// The DM text for one change. Plain text, so every Nostr client shows it.
pub fn message(change: &Change) -> String {
    match change {
        Change::Raised { node, finding, .. } => raised(node, finding),
        Change::Resolved { node, finding, .. } => format!(
            "✅ Firewatch: {} is no longer affected by {}\n{}\n\n{}",
            node.name, finding.advisory_id, finding.title, finding.reason
        ),
        Change::Unreachable { node } => format!(
            "⚠️ Firewatch: can't read {}, so it can't be checked.\n{}",
            node.name,
            node.error.as_deref().unwrap_or("unknown error")
        ),
        Change::Reachable { node } => {
            format!("Firewatch: {} can be read again.", node.name)
        }
    }
}

fn raised(node: &NodeReport, f: &Finding) -> String {
    let (icon, label) = match f.status {
        Status::Affected => ("🔴", "AFFECTED"),
        _ => ("🟠", "CAN'T TELL"),
    };
    let mut flags = vec![format!("{:?}", f.severity).to_lowercase()];
    if f.exploited_in_wild {
        flags.push("exploited in the wild".into());
    }
    let mut text = format!(
        "{icon} Firewatch: {} is {label}\n{} ({}): {}\n\n{}",
        node.name,
        f.advisory_id,
        flags.join(", "),
        f.title,
        f.reason
    );
    if f.status == Status::Affected {
        text.push_str(&format!("\n\nDevelopers say: \"{}\"", f.steps));
    }
    for check in &f.manual_checks {
        text.push_str(&format!("\n\nCheck by hand: {check}"));
    }
    text.push_str(&format!("\n\nSource: {}", f.source));
    text
}

#[cfg(test)]
mod tests {
    use firewatch_core::{Action, Implementation, Severity};

    use super::*;

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
}
