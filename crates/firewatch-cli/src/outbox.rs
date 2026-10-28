//! Alerts waiting for a connection (task A1).
//!
//! Every alert goes through the outbox, and every check tries to send what's
//! in it, oldest first, stopping at the first failure: when the internet is
//! down, one failed try per check is enough. Each node has at most one
//! findings message and one reachability message waiting, and the latest
//! state wins. A newer change is told against what the operator last heard,
//! so nobody gets a stale "red" for a node that has since been fixed, and a
//! node that went red and was fixed before anything got through gets one
//! message saying so.

use serde::{Deserialize, Serialize};

use firewatch_core::NodeReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// What changed in the node's findings.
    Findings,
    /// "Can't read this node."
    Unreachable,
    /// "This node can be read again."
    Reachable,
}

impl Kind {
    /// Which slot a message takes: one for findings, one for reachability.
    fn slot(self) -> u8 {
        match self {
            Kind::Findings => 0,
            Kind::Unreachable | Kind::Reachable => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub node: String,
    pub kind: Kind,
    /// The message itself, without the lines added when it's sent.
    pub text: String,
    /// When the change in `text` was seen.
    pub detected_at: u64,
    /// When this slot started waiting; kept when a newer change replaces it.
    pub waiting_since: u64,
    /// Set when the change was found by the first check after a restart.
    pub catch_up_since: Option<u64>,
    /// Findings only: the node as the operator last heard about it, which a
    /// newer change is told against. `None` for a node never reported.
    pub heard: Option<NodeReport>,
    /// Findings only: when the node went red while this waited.
    pub affected_since: Option<u64>,
    /// Set once a try to send failed while this waited, so it says when it
    /// happened: "Detected 14:02, sent 14:40 (no connection until then)."
    #[serde(default)]
    pub held: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Outbox {
    entries: Vec<Entry>,
}

impl Outbox {
    pub fn from_entries(entries: Vec<Entry>) -> Self {
        Outbox { entries }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The waiting message in this node's slot for `kind`.
    pub fn waiting(&self, node: &str, kind: Kind) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.node == node && e.kind.slot() == kind.slot())
    }

    /// Queues `entry`, replacing whatever waits in its slot but keeping that
    /// slot's place in the queue and how long it has waited.
    pub fn put(&mut self, mut entry: Entry) {
        match self
            .entries
            .iter_mut()
            .find(|e| e.node == entry.node && e.kind.slot() == entry.kind.slot())
        {
            Some(old) => {
                entry.waiting_since = old.waiting_since;
                entry.catch_up_since = entry.catch_up_since.or(old.catch_up_since);
                *old = entry;
            }
            None => self.entries.push(entry),
        }
    }

    /// Drops the message waiting in this node's slot for `kind`.
    pub fn cancel(&mut self, node: &str, kind: Kind) {
        self.entries
            .retain(|e| !(e.node == node && e.kind.slot() == kind.slot()));
    }

    /// The next message to send.
    pub fn first(&self) -> Option<&Entry> {
        self.entries.first()
    }

    /// Marks the first message as sent.
    pub fn sent(&mut self) {
        if !self.entries.is_empty() {
            self.entries.remove(0);
        }
    }

    /// Marks every waiting message as held back by a failed send.
    pub fn hold_all(&mut self) {
        for entry in &mut self.entries {
            entry.held = true;
        }
    }

    /// When the longest-waiting message started waiting.
    pub fn oldest(&self) -> Option<u64> {
        self.entries.iter().map(|e| e.waiting_since).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(node: &str, kind: Kind, text: &str, at: u64) -> Entry {
        Entry {
            node: node.into(),
            kind,
            text: text.into(),
            detected_at: at,
            waiting_since: at,
            catch_up_since: None,
            heard: None,
            affected_since: None,
            held: false,
        }
    }

    #[test]
    fn a_newer_message_replaces_the_waiting_one_in_place() {
        let mut outbox = Outbox::default();
        outbox.put(entry("a", Kind::Findings, "a red", 10));
        outbox.put(entry("b", Kind::Findings, "b red", 20));
        outbox.put(entry("a", Kind::Findings, "a amber", 30));
        assert_eq!(outbox.len(), 2);
        let first = outbox.first().unwrap();
        assert_eq!((first.text.as_str(), first.detected_at), ("a amber", 30));
        assert_eq!(first.waiting_since, 10);
        assert_eq!(outbox.oldest(), Some(10));
    }

    #[test]
    fn findings_and_reachability_wait_separately() {
        let mut outbox = Outbox::default();
        outbox.put(entry("a", Kind::Findings, "red", 10));
        outbox.put(entry("a", Kind::Unreachable, "down", 20));
        assert_eq!(outbox.len(), 2);
        outbox.cancel("a", Kind::Reachable);
        assert_eq!(outbox.len(), 1);
        assert!(outbox.waiting("a", Kind::Unreachable).is_none());
    }

    #[test]
    fn sent_removes_only_the_first() {
        let mut outbox = Outbox::default();
        outbox.put(entry("a", Kind::Findings, "1", 10));
        outbox.put(entry("b", Kind::Findings, "2", 20));
        outbox.sent();
        assert_eq!(outbox.first().unwrap().text, "2");
        outbox.sent();
        outbox.sent();
        assert_eq!(outbox.len(), 0);
        assert_eq!(outbox.oldest(), None);
    }
}
