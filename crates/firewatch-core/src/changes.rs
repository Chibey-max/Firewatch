//! What changed between two reports, i.e. what deserves an alert.
//!
//! An alert fires when a finding (or an unreadable node) turns red or amber,
//! and again when it turns green, so the operator hears "fixed" as well as
//! "broken". Nothing fires while a status stays the same.

use std::collections::HashMap;

use crate::engine::Status;
use crate::report::{Finding, NodeReport, Report};

#[derive(Debug, Clone, PartialEq)]
pub enum Change<'a> {
    /// A finding became Affected or Can't tell.
    Raised {
        node: &'a NodeReport,
        finding: &'a Finding,
        was: Option<Status>,
    },
    /// A finding that was Affected or Can't tell is now Not affected.
    Resolved {
        node: &'a NodeReport,
        finding: &'a Finding,
        was: Status,
    },
    /// The node could not be read, and could be (or was new) last time.
    Unreachable { node: &'a NodeReport },
    /// The node can be read again.
    Reachable { node: &'a NodeReport },
}

/// Changes from `prev` to `next`. With no `prev` (first run), every red or
/// amber finding and every unreachable node is reported.
pub fn changes<'a>(prev: Option<&Report>, next: &'a Report) -> Vec<Change<'a>> {
    let mut before: HashMap<(&str, &str), Status> = HashMap::new();
    let mut was_unreachable: HashMap<&str, bool> = HashMap::new();
    for node in prev.map(|r| r.nodes.as_slice()).unwrap_or_default() {
        was_unreachable.insert(&node.name, node.error.is_some());
        for f in &node.findings {
            before.insert((&node.name, &f.advisory_id), f.status);
        }
    }

    let mut out = Vec::new();
    for node in &next.nodes {
        match (
            node.error.is_some(),
            was_unreachable.get(node.name.as_str()),
        ) {
            (true, Some(true)) => continue,
            (true, _) => {
                out.push(Change::Unreachable { node });
                continue;
            }
            (false, Some(true)) => out.push(Change::Reachable { node }),
            _ => {}
        }
        for finding in &node.findings {
            let was = before
                .get(&(node.name.as_str(), finding.advisory_id.as_str()))
                .copied();
            if was == Some(finding.status) {
                continue;
            }
            match (finding.status, was) {
                (Status::NotAffected, Some(was @ (Status::Affected | Status::CantTell))) => {
                    out.push(Change::Resolved { node, finding, was })
                }
                (Status::NotAffected, _) => {}
                _ => out.push(Change::Raised { node, finding, was }),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisory::{Action, Implementation, Severity};

    fn finding(id: &str, status: Status) -> Finding {
        Finding {
            advisory_id: id.into(),
            title: "t".into(),
            severity: Severity::Critical,
            exploited_in_wild: false,
            verified: true,
            fixed: None,
            source: "https://example.com".into(),
            status,
            reason: "r".into(),
            manual_checks: vec![],
            action: Action::Upgrade,
            steps: "s".into(),
        }
    }

    fn report(findings: Vec<Finding>) -> Report {
        Report {
            checked_at: 0,
            nodes: vec![NodeReport {
                name: "n".into(),
                implementation: Implementation::Cln,
                version: Some("v26.06.7".into()),
                network: None,
                status: Status::Affected,
                error: None,
                build_check: Default::default(),
                exposure: vec![],
                findings,
            }],
        }
    }

    fn unreachable() -> Report {
        Report {
            checked_at: 0,
            nodes: vec![NodeReport::unreachable(
                "n",
                Implementation::Cln,
                "down".into(),
            )],
        }
    }

    #[test]
    fn first_run_raises_red_and_amber_only() {
        let next = report(vec![
            finding("A", Status::Affected),
            finding("B", Status::CantTell),
            finding("C", Status::NotAffected),
        ]);
        let ids: Vec<&str> = changes(None, &next)
            .iter()
            .map(|c| match c {
                Change::Raised { finding, .. } => finding.advisory_id.as_str(),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(ids, ["A", "B"]);
    }

    #[test]
    fn unchanged_status_is_silent() {
        let r = report(vec![finding("A", Status::Affected)]);
        assert!(changes(Some(&r), &r).is_empty());
    }

    #[test]
    fn upgrade_resolves() {
        let prev = report(vec![finding("A", Status::Affected)]);
        let next = report(vec![finding("A", Status::NotAffected)]);
        assert!(matches!(
            changes(Some(&prev), &next).as_slice(),
            [Change::Resolved {
                was: Status::Affected,
                ..
            }]
        ));
    }

    #[test]
    fn amber_to_red_raises_again() {
        let prev = report(vec![finding("A", Status::CantTell)]);
        let next = report(vec![finding("A", Status::Affected)]);
        assert!(matches!(
            changes(Some(&prev), &next).as_slice(),
            [Change::Raised {
                was: Some(Status::CantTell),
                ..
            }]
        ));
    }

    #[test]
    fn unreachable_alerts_once_then_recovers() {
        let up = report(vec![finding("A", Status::Affected)]);
        let down = unreachable();
        assert!(matches!(
            changes(Some(&up), &down).as_slice(),
            [Change::Unreachable { .. }]
        ));
        assert!(changes(Some(&down), &down).is_empty());

        // Back up: reachable again, and the still-red finding is re-raised
        // because it was not visible while the node was down.
        let back = changes(Some(&down), &up);
        assert!(matches!(
            back.as_slice(),
            [Change::Reachable { .. }, Change::Raised { was: None, .. }]
        ));
    }
}
