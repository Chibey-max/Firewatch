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
    NodeInfo {
        name: "test".into(),
        implementation,
        version: version.into(),
        network: Some("regtest".into()),
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
    "FW-2026-004",
    "FW-2026-005",
    "FW-2026-006",
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
        assert!(a.source.starts_with("https://github.com/"), "{}", a.id);
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
fn fw_2026_004_cln_go_offline() {
    use Implementation::Cln;
    assert_eq!(status("FW-2026-004", Cln, "v26.04.1"), Status::Affected);
    assert_eq!(status("FW-2026-004", Cln, "v26.06rc1"), Status::Affected);
    assert_eq!(status("FW-2026-004", Cln, "v26.06"), Status::NotAffected);
    assert!(!advisory("FW-2026-004").verified);
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
