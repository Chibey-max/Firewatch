//! The report shape shared by the CLI, alerts and the dashboard JSON API.

use serde::{Deserialize, Serialize};

use crate::advisory::{Action, Advisory, Implementation, Severity};
use crate::engine::{evaluate, Status};
use crate::node::NodeInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Unix seconds.
    pub checked_at: u64,
    pub nodes: Vec<NodeReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeReport {
    pub name: String,
    pub implementation: Implementation,
    /// `None` when the node could not be read.
    pub version: Option<String>,
    pub network: Option<String>,
    /// The worst status across findings; "not_affected" if there are none,
    /// "cant_tell" if the node could not be read.
    pub status: Status,
    /// Why the node could not be read, if it could not.
    pub error: Option<String>,
    /// Worst first.
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub advisory_id: String,
    pub title: String,
    pub severity: Severity,
    pub exploited_in_wild: bool,
    pub verified: bool,
    pub fixed: Option<String>,
    pub source: String,
    pub status: Status,
    pub reason: String,
    pub manual_checks: Vec<String>,
    /// The version is inside the affected range; see [`crate::Verdict::in_range`].
    #[serde(default)]
    pub in_range: bool,
    pub action: Action,
    /// Quoted from `source`.
    pub steps: String,
}

impl NodeReport {
    pub fn from_node(node: &NodeInfo, advisories: &[Advisory]) -> Self {
        let mut findings: Vec<Finding> = advisories
            .iter()
            .filter_map(|a| evaluate(node, a).map(|v| (a, v)))
            .map(|(a, v)| Finding {
                advisory_id: v.advisory_id,
                title: a.title.clone(),
                severity: a.severity,
                exploited_in_wild: a.exploited_in_wild,
                verified: a.verified,
                fixed: a.fixed.clone(),
                source: a.source.clone(),
                status: v.status,
                reason: v.reason,
                manual_checks: v.manual_checks,
                in_range: v.in_range,
                action: a.remediation.action,
                steps: a.remediation.steps.clone(),
            })
            .collect();
        findings.sort_by(|a, b| {
            (b.status, b.severity)
                .cmp(&(a.status, a.severity))
                .then_with(|| a.advisory_id.cmp(&b.advisory_id))
        });
        NodeReport {
            name: node.name.clone(),
            implementation: node.implementation,
            version: Some(node.version.clone()),
            network: node.network.clone(),
            status: findings
                .iter()
                .map(|f| f.status)
                .max()
                .unwrap_or(Status::NotAffected),
            error: None,
            findings,
        }
    }

    /// A node Firewatch could not read. Never green: we know nothing about it.
    pub fn unreachable(name: &str, implementation: Implementation, error: String) -> Self {
        NodeReport {
            name: name.to_string(),
            implementation,
            version: None,
            network: None,
            status: Status::CantTell,
            error: Some(error),
            findings: vec![],
        }
    }
}

impl Report {
    pub fn build(nodes: &[NodeInfo], advisories: &[Advisory], checked_at: u64) -> Self {
        Report {
            checked_at,
            nodes: nodes
                .iter()
                .map(|n| NodeReport::from_node(n, advisories))
                .collect(),
        }
    }

    pub fn worst(&self) -> Status {
        self.nodes
            .iter()
            .map(|n| n.status)
            .max()
            .unwrap_or(Status::NotAffected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_node_is_never_green() {
        let r = NodeReport::unreachable("x", Implementation::Cln, "timeout".into());
        assert_eq!(r.status, Status::CantTell);
        let report = Report {
            checked_at: 0,
            nodes: vec![r],
        };
        assert_eq!(report.worst(), Status::CantTell);
    }
}
