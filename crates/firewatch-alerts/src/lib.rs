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

pub mod lang;

pub use lang::{fill, Language, Strings, EN};
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

/// Checks that `recipient` is a Nostr public key (npub or hex) before it's
/// written to a config.
pub fn check_recipient(recipient: &str) -> Result<(), AlertError> {
    PublicKey::parse(recipient.trim())
        .map(|_| ())
        .map_err(|e| AlertError::Config(format!("not a Nostr public key: {e}")))
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
