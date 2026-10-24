//! The matching engine: one node and one advisory in, one verdict out.
//!
//! Rules, in order:
//! 1. An advisory for another implementation does not apply (no verdict).
//! 2. An unparseable node version is "Can't tell".
//! 3. A version outside the affected range is "Not affected".
//! 4. Any condition confirmed false is "Not affected".
//! 5. Any condition that can't be read is "Can't tell".
//! 6. Otherwise "Affected".

use serde::{Deserialize, Serialize};

use crate::advisory::{Advisory, Condition};
use crate::node::NodeInfo;
use crate::version::Version;

/// Ordered so that `max()` gives the worst status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    NotAffected,
    CantTell,
    Affected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub advisory_id: String,
    pub status: Status,
    /// One sentence the operator can read, shown next to the colour.
    pub reason: String,
    /// What to check by hand when the status is "Can't tell".
    pub manual_checks: Vec<String>,
    /// The node's version is inside the affected range. False when it is
    /// outside or can't be parsed. A "Not affected" verdict with this set was
    /// ruled out by a condition, not by the version.
    pub in_range: bool,
}

enum Outcome {
    True,
    False(String),
    Unknown(String),
}

/// Returns `None` when the advisory is for a different implementation.
pub fn evaluate(node: &NodeInfo, advisory: &Advisory) -> Option<Verdict> {
    if node.implementation != advisory.implementation {
        return None;
    }
    let verdict = |status, in_range, reason: String, manual_checks| Verdict {
        advisory_id: advisory.id.clone(),
        status,
        reason,
        manual_checks,
        in_range,
    };
    let range = &advisory.affected;

    let version = match Version::parse(&node.version) {
        Ok(v) => v,
        Err(_) => {
            return Some(verdict(
                Status::CantTell,
                false,
                format!(
                    "Could not parse the node's version '{}'; compare it by hand with the affected range {range}.",
                    node.version
                ),
                vec![format!("Check whether version '{}' is {range}.", node.version)],
            ));
        }
    };

    if !range.matches(&version) {
        let has_fix = advisory
            .fixed_versions()
            .unwrap_or_default()
            .iter()
            .any(|fixed| version >= *fixed);
        let reason = match (&advisory.fixed, has_fix) {
            (Some(fixed), true) => {
                format!("Version {version} includes the fix (fixed in {fixed}).")
            }
            _ => format!("Version {version} is outside the affected range {range}."),
        };
        return Some(verdict(Status::NotAffected, false, reason, vec![]));
    }

    let outcomes: Vec<Outcome> = advisory
        .conditions
        .iter()
        .map(|c| check_condition(node, c))
        .collect();

    let ruled_out: Vec<&str> = outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::False(why) => Some(why.as_str()),
            _ => None,
        })
        .collect();
    if !ruled_out.is_empty() {
        return Some(verdict(
            Status::NotAffected,
            true,
            format!(
                "Version {version} is in the affected range {range}, but {}.",
                ruled_out.join("; ")
            ),
            vec![],
        ));
    }

    let unknown: Vec<String> = outcomes
        .into_iter()
        .filter_map(|o| match o {
            Outcome::Unknown(hint) => Some(hint),
            _ => None,
        })
        .collect();
    if !unknown.is_empty() {
        let n = unknown.len();
        return Some(verdict(
            Status::CantTell,
            true,
            format!(
                "Version {version} is in the affected range {range}, but {n} condition{} can't be checked over RPC.",
                if n == 1 { "" } else { "s" }
            ),
            unknown,
        ));
    }

    let fix = match &advisory.fixed {
        Some(fixed) => format!(" Fixed in {fixed}."),
        None => " No fixed release yet.".to_string(),
    };
    Some(verdict(
        Status::Affected,
        true,
        format!("Version {version} is in the affected range {range}.{fix}"),
        vec![],
    ))
}

fn check_condition(node: &NodeInfo, condition: &Condition) -> Outcome {
    match condition {
        Condition::VersionRange => Outcome::True,
        Condition::ConfigEnabled { option } => match &node.config {
            None => Outcome::Unknown(format!(
                "Could not read the node's config; check whether '{option}' is enabled."
            )),
            Some(config) => match config.get(option) {
                Some(true) => Outcome::True,
                Some(false) => Outcome::False(format!("'{option}' is disabled")),
                None => Outcome::Unknown(format!(
                    "The node did not report '{option}'; check whether it is enabled."
                )),
            },
        },
        Condition::FeatureBit { bit } => match &node.features {
            None => Outcome::Unknown(format!(
                "Could not read the node's feature bits; check whether feature bit {bit} is set."
            )),
            Some(bits) if bits.contains(bit) || bits.contains(&(bit ^ 1)) => Outcome::True,
            Some(_) => Outcome::False(format!("feature bit {bit} is not set")),
        },
        Condition::Network { networks } => match node.network.as_deref() {
            None => Outcome::Unknown(format!(
                "Could not read the node's network; this only applies on {}.",
                networks.join(" or ")
            )),
            Some(network) if networks.iter().any(|n| same_network(n, network)) => Outcome::True,
            Some(network) => Outcome::False(format!(
                "it only applies on {}, and this node runs on {network}",
                networks.join(" or ")
            )),
        },
        Condition::Manual { check_hint } => Outcome::Unknown(check_hint.clone()),
    }
}

/// CLN calls mainnet `bitcoin`; LND calls it `mainnet`.
fn same_network(a: &str, b: &str) -> bool {
    let canonical = |n: &str| match n.to_ascii_lowercase().as_str() {
        "bitcoin" | "mainnet" => "mainnet".to_string(),
        other => other.to_string(),
    };
    canonical(a) == canonical(b)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use super::*;
    use crate::advisory::Implementation;

    fn advisory(conditions: &str) -> Advisory {
        let text = format!(
            r#"
            id = "FW-TEST-001"
            implementation = "cln"
            title = "Test"
            affected = "< 26.06.8"
            fixed = "26.06.8"
            severity = "high"
            source = "https://example.com"
            reviewed_by = "Dave"
            {conditions}
            [remediation]
            action = "upgrade"
            steps = "Upgrade."
            "#
        );
        Advisory::from_toml(Path::new("test.toml"), &text).unwrap()
    }

    fn node(version: &str) -> NodeInfo {
        NodeInfo {
            name: "test".into(),
            implementation: Implementation::Cln,
            version: version.into(),
            network: Some("regtest".into()),
            config: None,
            features: None,
            commit_hash: None,
        }
    }

    fn status(node: &NodeInfo, advisory: &Advisory) -> Status {
        evaluate(node, advisory).unwrap().status
    }

    const SPLICING: &str = r#"
        [[conditions]]
        kind = "config_enabled"
        option = "experimental-splicing"
    "#;

    #[test]
    fn version_only() {
        let a = advisory("");
        assert_eq!(status(&node("v26.06.7"), &a), Status::Affected);
        assert_eq!(status(&node("v26.06.8"), &a), Status::NotAffected);
        assert_eq!(status(&node("v26.06.8rc1"), &a), Status::Affected);
    }

    #[test]
    fn unparseable_version_is_cant_tell() {
        let v = evaluate(&node("custom-build"), &advisory("")).unwrap();
        assert_eq!(v.status, Status::CantTell);
        assert_eq!(v.manual_checks.len(), 1);
    }

    #[test]
    fn other_implementation_does_not_apply() {
        let mut n = node("0.20.0-beta");
        n.implementation = Implementation::Lnd;
        assert!(evaluate(&n, &advisory("")).is_none());
    }

    #[test]
    fn fixed_reason_mentions_fix() {
        let v = evaluate(&node("v26.06.9"), &advisory("")).unwrap();
        assert_eq!(
            v.reason,
            "Version 26.06.9 includes the fix (fixed in 26.06.8)."
        );
    }

    #[test]
    fn config_condition() {
        let a = advisory(SPLICING);
        let mut n = node("v26.06.7");
        assert_eq!(status(&n, &a), Status::CantTell, "unreadable config");

        n.config = Some(BTreeMap::new());
        assert_eq!(status(&n, &a), Status::CantTell, "option not reported");

        n.config = Some(BTreeMap::from([("experimental-splicing".into(), false)]));
        assert_eq!(status(&n, &a), Status::NotAffected);

        n.config = Some(BTreeMap::from([("experimental-splicing".into(), true)]));
        assert_eq!(status(&n, &a), Status::Affected);
    }

    #[test]
    fn feature_bit_matches_either_bit_of_pair() {
        let a = advisory("[[conditions]]\nkind = \"feature_bit\"\nbit = 62\n");
        let mut n = node("v26.06.7");
        assert_eq!(status(&n, &a), Status::CantTell);
        n.features = Some(BTreeSet::from([63]));
        assert_eq!(status(&n, &a), Status::Affected);
        n.features = Some(BTreeSet::from([12, 13]));
        assert_eq!(status(&n, &a), Status::NotAffected);
    }

    #[test]
    fn network_condition() {
        let a = advisory("[[conditions]]\nkind = \"network\"\nnetworks = [\"mainnet\"]\n");
        let mut n = node("v26.06.7");
        assert_eq!(status(&n, &a), Status::NotAffected, "regtest");
        n.network = Some("bitcoin".into());
        assert_eq!(status(&n, &a), Status::Affected, "CLN's name for mainnet");
        n.network = None;
        assert_eq!(status(&n, &a), Status::CantTell);
    }

    #[test]
    fn manual_condition_is_cant_tell_with_hint() {
        let a = advisory("[[conditions]]\nkind = \"manual\"\ncheck_hint = \"Check your peers.\"\n");
        let v = evaluate(&node("v26.06.7"), &a).unwrap();
        assert_eq!(v.status, Status::CantTell);
        assert_eq!(v.manual_checks, vec!["Check your peers.".to_string()]);
    }

    #[test]
    fn confirmed_false_beats_unknown() {
        let a = advisory(&format!(
            "{SPLICING}\n[[conditions]]\nkind = \"manual\"\ncheck_hint = \"x\"\n"
        ));
        let mut n = node("v26.06.7");
        n.config = Some(BTreeMap::from([("experimental-splicing".into(), false)]));
        assert_eq!(status(&n, &a), Status::NotAffected);
    }

    #[test]
    fn in_range_says_whether_the_version_matched() {
        let a = advisory(SPLICING);
        let mut n = node("v26.06.7");
        n.config = Some(BTreeMap::from([("experimental-splicing".into(), false)]));
        let ruled_out = evaluate(&n, &a).unwrap();
        assert_eq!(ruled_out.status, Status::NotAffected);
        assert!(
            ruled_out.in_range,
            "ruled out by a condition, not the version"
        );

        assert!(
            !evaluate(&node("v26.06.8"), &a).unwrap().in_range,
            "outside the range"
        );
        assert!(
            !evaluate(&node("custom-build"), &a).unwrap().in_range,
            "unparseable"
        );
        assert!(
            evaluate(&node("v26.06.7"), &a).unwrap().in_range,
            "can't tell"
        );
    }

    #[test]
    fn out_of_range_never_checks_conditions() {
        let a = advisory("[[conditions]]\nkind = \"manual\"\ncheck_hint = \"x\"\n");
        assert_eq!(status(&node("v26.06.8"), &a), Status::NotAffected);
    }
}
