//! One test per advisory in `advisories/`, checked against the versions the
//! demo uses. A new advisory file without a test here fails `all_have_tests`.

use std::path::PathBuf;

use firewatch_core::{evaluate, load_dir, Advisory, Implementation, NodeInfo, Status};

fn advisories() -> Vec<Advisory> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../advisories");
    load_dir(&dir).expect("advisories/ must load cleanly")
}

fn advisory(id: &str) -> Advisory {
    advisories()
        .into_iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("{id} missing"))
}

fn node(implementation: Implementation, version: &str) -> NodeInfo {
    on_network(implementation, version, "regtest")
}

fn on_network(implementation: Implementation, version: &str, network: &str) -> NodeInfo {
    NodeInfo {
        name: "test".into(),
        implementation,
        version: version.into(),
        network: Some(network.into()),
        config: None,
        features: None,
    }
}

fn status(id: &str, implementation: Implementation, version: &str) -> Status {
    evaluate(&node(implementation, version), &advisory(id))
        .unwrap()
        .status
}

const TESTED: &[&str] = &[
    "FW-2026-001",
    "FW-2026-002",
    "FW-2026-003",
    "FW-2026-005",
    "FW-2026-006",
    "FW-2026-007",
    "FW-2026-008",
    "FW-2026-009",
    "FW-2026-010",
    "FW-2026-011",
    "FW-2026-012",
    "FW-2026-013",
];

#[test]
fn all_have_tests() {
    let ids: Vec<String> = advisories().into_iter().map(|a| a.id).collect();
    assert_eq!(
        ids, TESTED,
        "add a test for each new advisory and list it in TESTED"
    );
}

#[test]
fn verified_advisories_quote_real_sources() {
    for a in advisories().into_iter().filter(|a| a.verified) {
        assert!(!a.remediation.steps.contains("UNVERIFIED"), "{}", a.id);
        assert!(
            a.source.starts_with("https://github.com/")
                || a.source
                    .starts_with("https://security.lightning.engineering/"),
            "{}",
            a.id
        );
    }
}

#[test]
fn fw_2026_001_ldk_splice_fee() {
    use Implementation::Ldk;
    assert_eq!(status("FW-2026-001", Ldk, "0.2.5"), Status::CantTell);
    assert_eq!(status("FW-2026-001", Ldk, "0.2.6"), Status::NotAffected);
}

#[test]
fn fw_2026_002_ldk_bogus_htlc() {
    use Implementation::Ldk;
    assert_eq!(status("FW-2026-002", Ldk, "0.2.5"), Status::CantTell);
    assert_eq!(status("FW-2026-002", Ldk, "0.2.6"), Status::NotAffected);
}

#[test]
fn fw_2026_003_cln_26_06_8() {
    use Implementation::Cln;
    assert_eq!(status("FW-2026-003", Cln, "v26.06.7"), Status::Affected);
    assert_eq!(status("FW-2026-003", Cln, "v26.06.8rc1"), Status::Affected);
    assert_eq!(status("FW-2026-003", Cln, "v26.06.8"), Status::NotAffected);
    assert_eq!(status("FW-2026-003", Cln, "v26.06.9"), Status::NotAffected);
}

#[test]
fn fw_2026_005_lnd_coop_close_reorg() {
    use Implementation::Lnd;
    assert_eq!(status("FW-2026-005", Lnd, "0.20.1-beta"), Status::CantTell);
    assert_eq!(
        status("FW-2026-005", Lnd, "0.21.0-beta.rc1"),
        Status::CantTell
    );
    assert_eq!(
        status("FW-2026-005", Lnd, "0.21.0-beta"),
        Status::NotAffected
    );
    assert!(!advisory("FW-2026-005").verified);
}

#[test]
fn fw_2026_006_cln_26_06_9() {
    use Implementation::Cln;
    assert_eq!(status("FW-2026-006", Cln, "v26.06.7"), Status::Affected);
    assert_eq!(status("FW-2026-006", Cln, "v26.06.8"), Status::Affected);
    assert_eq!(status("FW-2026-006", Cln, "v26.06.9"), Status::NotAffected);
}

#[test]
fn fw_2026_007_lnd_gossip_stall() {
    use Implementation::Lnd;
    assert_eq!(
        status("FW-2026-007", Lnd, "0.18.5-beta"),
        Status::NotAffected
    );
    assert_eq!(status("FW-2026-007", Lnd, "0.19.0-beta"), Status::Affected);
    assert_eq!(
        status("FW-2026-007", Lnd, "0.20.0-beta commit=v0.20.0-beta"),
        Status::Affected
    );
    assert_eq!(
        status("FW-2026-007", Lnd, "0.20.1-beta"),
        Status::NotAffected
    );
}

#[test]
fn fw_2026_008_lnd_zero_timestamp_panic() {
    use Implementation::Lnd;
    assert_eq!(status("FW-2026-008", Lnd, "0.16.4-beta"), Status::Affected);
    assert_eq!(status("FW-2026-008", Lnd, "0.20.0-beta"), Status::Affected);
    assert_eq!(
        status("FW-2026-008", Lnd, "0.20.1-beta"),
        Status::NotAffected
    );
}

#[test]
fn fw_2026_009_lnd_dns_seed_panic() {
    use Implementation::Lnd;
    let a = advisory("FW-2026-009");
    let on = |version, network| {
        evaluate(&on_network(Lnd, version, network), &a)
            .unwrap()
            .status
    };
    assert_eq!(
        on("0.20.0-beta", "regtest"),
        Status::NotAffected,
        "no DNS seeds off mainnet"
    );
    assert_eq!(
        on("0.20.0-beta", "mainnet"),
        Status::CantTell,
        "nobootstrap is in lnd.conf"
    );
    assert_eq!(on("0.21.0-beta", "mainnet"), Status::CantTell);
    assert_eq!(on("0.20.2-beta", "mainnet"), Status::NotAffected);
    assert_eq!(on("0.21.1-beta", "mainnet"), Status::NotAffected);
}

#[test]
fn fw_2026_010_lnd_update_fee_breach() {
    use Implementation::Lnd;
    assert_eq!(status("FW-2026-010", Lnd, "0.17.5-beta"), Status::Affected);
    assert_eq!(status("FW-2026-010", Lnd, "0.18.2-beta"), Status::Affected);
    assert_eq!(
        status("FW-2026-010", Lnd, "0.18.3-beta"),
        Status::NotAffected
    );
    assert_eq!(
        status("FW-2026-010", Lnd, "0.18.5-beta"),
        Status::NotAffected
    );
}

#[test]
fn fw_2026_011_lnd_pending_commitment_failback() {
    use Implementation::Lnd;
    assert_eq!(status("FW-2026-011", Lnd, "0.17.5-beta"), Status::Affected);
    assert_eq!(
        status("FW-2026-011", Lnd, "0.18.0-beta"),
        Status::NotAffected
    );
}

#[test]
fn fw_2026_012_lnd_htlc_sweep_budget() {
    use Implementation::Lnd;
    assert_eq!(
        status("FW-2026-012", Lnd, "0.17.5-beta"),
        Status::NotAffected
    );
    assert_eq!(status("FW-2026-012", Lnd, "0.18.0-beta"), Status::Affected);
    assert_eq!(status("FW-2026-012", Lnd, "0.18.5-beta"), Status::Affected);
    assert_eq!(
        status("FW-2026-012", Lnd, "0.19.0-beta"),
        Status::NotAffected
    );
}

#[test]
fn fw_2026_013_lnd_interceptor_cancel_settles_invoice() {
    use Implementation::Lnd;
    assert_eq!(
        status("FW-2026-013", Lnd, "0.18.3-beta"),
        Status::NotAffected
    );
    assert_eq!(status("FW-2026-013", Lnd, "0.18.4-beta"), Status::CantTell);
    assert_eq!(status("FW-2026-013", Lnd, "0.18.5-beta"), Status::CantTell);
    assert_eq!(
        status("FW-2026-013", Lnd, "0.19.0-beta"),
        Status::NotAffected
    );
}
