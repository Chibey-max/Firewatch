//! The report shape shared by the CLI, alerts and the dashboard JSON API.

use serde::{Deserialize, Serialize};

use crate::advisory::{Action, Advisory, Implementation, Severity};
use crate::engine::{evaluate, Status};
use crate::exposure::Exposure;
use crate::node::NodeInfo;
use crate::releases::{build_check, short_commit, BuildCheck, ReleaseTable};

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
    /// Whether the binary is the official release its version claims.
    #[serde(default)]
    pub build_check: BuildCheck,
    /// Where the node's APIs listen, most exposed first. Never changes
    /// `status`: a patched node can still be exposed. See `exposure.rs`.
    #[serde(default)]
    pub exposure: Vec<Exposure>,
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
    /// `releases` enables the build check for its implementation.
    pub fn from_node(
        node: &NodeInfo,
        advisories: &[Advisory],
        releases: Option<&ReleaseTable>,
    ) -> Self {
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
        let build_check = build_check(node, releases);
        if let BuildCheck::Unreleased { release, build } = &build_check {
            // Commits after the tag may add or remove any fix, so no
            // version-based verdict holds.
            let reason = format!(
                "This node reports version {release}, but its build (\"{build}\") is from a commit after the {release} release, so it may hold changes or fixes the release doesn't. Firewatch can't trust the version number."
            );
            for f in &mut findings {
                f.status = Status::CantTell;
                f.reason = reason.clone();
                f.manual_checks = vec![
                    "Confirm which commit this binary was built from, or install the official release."
                        .into(),
                ];
            }
        }
        if let BuildCheck::Modified { release, build } = &build_check {
            // The release's own code is most likely there, so a red finding
            // stays red; but a fix could have been changed, so nothing is green.
            for f in findings
                .iter_mut()
                .filter(|f| f.status == Status::NotAffected)
            {
                f.status = Status::CantTell;
                f.reason = format!(
                    "{release} includes this fix, but this build (\"{build}\") was made from a source tree with local changes, so Firewatch can't confirm the fix is intact."
                );
                f.manual_checks = vec![format!(
                    "If you built {release} from source, run `git status` in the source tree: Core Lightning marks a clean build \"-modded\" when the build only regenerated files. Otherwise, install the official release."
                )];
            }
        }
        if let BuildCheck::Mismatch {
            release,
            expected,
            actual,
            build,
        } = &build_check
        {
            let reason = if expected.eq_ignore_ascii_case(actual) {
                format!(
                    "This node reports version {release} and the official commit, but describes its build as \"{}\", not {release}, so its code may have been changed. Firewatch can't trust the version number.",
                    build.as_deref().unwrap_or_default()
                )
            } else {
                format!(
                    "This node reports version {release}, but its build ({}) is not the official {release} release ({}). Firewatch can't trust the version number.",
                    short_commit(actual),
                    short_commit(expected)
                )
            };
            // The version number is untrusted, so no version-based verdict is.
            for f in &mut findings {
                f.status = Status::CantTell;
                f.reason = reason.clone();
                f.manual_checks =
                    vec!["Confirm where this binary came from, or reinstall it from the official release.".into()];
            }
        }
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
                .chain(
                    matches!(
                        build_check,
                        BuildCheck::Mismatch { .. } | BuildCheck::Unreleased { .. }
                    )
                    .then_some(Status::CantTell),
                )
                .max()
                .unwrap_or(Status::NotAffected),
            error: None,
            build_check,
            exposure: crate::exposure::check(&node.apis),
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
            build_check: BuildCheck::NotChecked,
            exposure: vec![],
            findings: vec![],
        }
    }
}

impl Report {
    pub fn build(
        nodes: &[NodeInfo],
        advisories: &[Advisory],
        releases: Option<&ReleaseTable>,
        checked_at: u64,
    ) -> Self {
        Report {
            checked_at,
            nodes: nodes
                .iter()
                .map(|n| NodeReport::from_node(n, advisories, releases))
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

    fn lnd(version: &str, commit: &str) -> NodeInfo {
        NodeInfo {
            name: "lnd".into(),
            implementation: Implementation::Lnd,
            version: version.into(),
            network: Some("regtest".into()),
            config: None,
            features: None,
            commit_hash: Some(commit.into()),
            apis: vec![],
            warnings: vec![],
        }
    }

    fn table() -> ReleaseTable {
        toml::from_str(
            r#"
            implementation = "lnd"
            [[release]]
            version = "v0.20.0-beta"
            commit = "b9ea7070c20ad2ca8514a47d9b4d560a501f0487"
            "#,
        )
        .unwrap()
    }

    fn advisories() -> Vec<Advisory> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../advisories");
        crate::advisory::load_dir(&dir).unwrap()
    }

    #[test]
    fn official_build_keeps_normal_verdicts() {
        let node = lnd("0.20.0-beta", "b9ea7070c20ad2ca8514a47d9b4d560a501f0487");
        let r = NodeReport::from_node(&node, &advisories(), Some(&table()));
        assert_eq!(r.build_check, BuildCheck::Match);
        assert_eq!(r.status, Status::Affected);
        assert!(r.findings.iter().any(|f| f.status == Status::NotAffected));
    }

    #[test]
    fn mismatched_build_makes_every_finding_cant_tell() {
        let node = lnd(
            "0.20.0-beta commit=v0.20.0-beta-1-g1234567",
            "1234567890abcdef1234567890abcdef12345678",
        );
        let r = NodeReport::from_node(&node, &advisories(), Some(&table()));
        assert!(matches!(r.build_check, BuildCheck::Mismatch { .. }));
        assert_eq!(r.status, Status::CantTell);
        assert!(!r.findings.is_empty());
        for f in &r.findings {
            assert_eq!(f.status, Status::CantTell, "{}", f.advisory_id);
            assert!(f
                .reason
                .contains("(1234567) is not the official v0.20.0-beta release (b9ea707)"));
            assert_eq!(f.manual_checks.len(), 1);
        }
    }

    #[test]
    fn changed_build_on_the_official_commit_makes_every_finding_cant_tell() {
        let node = lnd(
            "0.20.0-beta commit=v0.20.0-beta-dirty",
            "b9ea7070c20ad2ca8514a47d9b4d560a501f0487",
        );
        let r = NodeReport::from_node(&node, &advisories(), Some(&table()));
        assert!(matches!(r.build_check, BuildCheck::Mismatch { .. }));
        assert_eq!(r.status, Status::CantTell);
        for f in &r.findings {
            assert_eq!(f.status, Status::CantTell, "{}", f.advisory_id);
            assert!(f
                .reason
                .contains("describes its build as \"v0.20.0-beta-dirty\""));
        }
    }

    fn cln(version: &str) -> NodeReport {
        NodeReport::from_node(&cln_node(version), &advisories(), Some(&table()))
    }

    fn cln_node(version: &str) -> NodeInfo {
        NodeInfo {
            name: "cln".into(),
            implementation: Implementation::Cln,
            version: version.into(),
            network: Some("regtest".into()),
            config: None,
            features: None,
            commit_hash: None,
            apis: vec![],
            warnings: vec![],
        }
    }

    fn status_of(r: &NodeReport, id: &str) -> Status {
        r.findings
            .iter()
            .find(|f| f.advisory_id == id)
            .unwrap()
            .status
    }

    #[test]
    fn cln_release_keeps_normal_verdicts() {
        let r = cln("v26.06.7");
        assert_eq!(r.build_check, BuildCheck::NotChecked);
        assert_eq!(status_of(&r, "FW-2026-003"), Status::Affected);
        assert_eq!(cln("v26.06.9").status, Status::NotAffected);
    }

    #[test]
    fn cln_unreleased_build_makes_every_finding_cant_tell() {
        for version in ["v26.06.7-12-gabc1234", "v26.06.9-3-gdef5678"] {
            let r = cln(version);
            assert!(matches!(r.build_check, BuildCheck::Unreleased { .. }));
            assert_eq!(r.status, Status::CantTell, "{version}");
            for f in &r.findings {
                assert_eq!(f.status, Status::CantTell, "{version} {}", f.advisory_id);
                assert!(f.reason.contains("is from a commit after the"));
            }
        }
    }

    #[test]
    fn cln_modified_build_keeps_red_but_is_never_green() {
        // 26.06.7 is affected by both CLN advisories: they stay red.
        let r = cln("v26.06.7-modded");
        assert!(matches!(r.build_check, BuildCheck::Modified { .. }));
        assert_eq!(r.status, Status::Affected);
        assert_eq!(status_of(&r, "FW-2026-003"), Status::Affected);
        assert_eq!(status_of(&r, "FW-2026-006"), Status::Affected);

        // 26.06.8 has 003's fix but not 006's: 006 stays red, 003 can't be green.
        let r = cln("v26.06.8-modded");
        assert_eq!(status_of(&r, "FW-2026-006"), Status::Affected);
        assert_eq!(status_of(&r, "FW-2026-003"), Status::CantTell);

        // 26.06.9 has both fixes: nothing is green, so the node is amber.
        let r = cln("v26.06.9-modded");
        assert_eq!(r.status, Status::CantTell);
        for f in &r.findings {
            assert_eq!(f.status, Status::CantTell, "{}", f.advisory_id);
            assert!(f.reason.contains("can't confirm the fix is intact"));
            assert!(f.manual_checks[0].contains("git status"));
        }
    }

    #[test]
    fn exposure_never_changes_the_verdict() {
        // A patched node on Polar: clnrest on 0.0.0.0, yet still green.
        let node = NodeInfo {
            apis: vec![crate::node::ApiListen {
                api: crate::node::Api::Rest,
                listen: crate::node::Listen::Read {
                    addresses: vec!["0.0.0.0:8080".into()],
                },
            }],
            ..cln_node("v26.06.9")
        };
        let r = NodeReport::from_node(&node, &advisories(), Some(&table()));
        assert_eq!(r.status, Status::NotAffected);
        assert_eq!(r.exposure.len(), 1);
        assert_eq!(
            r.exposure[0].status,
            crate::exposure::ExposureStatus::Exposed
        );

        // The same node without listen data: everything but `exposure` is
        // identical, so no field can pick the exposure up.
        let without = NodeReport::from_node(&cln_node("v26.06.9"), &advisories(), Some(&table()));
        let strip = |r: &NodeReport| {
            let mut v = serde_json::to_value(r).unwrap();
            v.as_object_mut().unwrap().remove("exposure");
            v
        };
        assert_eq!(strip(&r), strip(&without));
    }

    #[test]
    fn an_unreadable_node_has_no_exposure_entries() {
        let r = NodeReport::unreachable("x", Implementation::Cln, "down".into());
        assert!(r.exposure.is_empty());
    }

    #[test]
    fn without_a_table_nothing_is_checked() {
        let node = lnd("0.20.0-beta", "1234567890abcdef1234567890abcdef12345678");
        let r = NodeReport::from_node(&node, &advisories(), None);
        assert_eq!(r.build_check, BuildCheck::NotChecked);
        assert_eq!(r.status, Status::Affected);
    }
}
