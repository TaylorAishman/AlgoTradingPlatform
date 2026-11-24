//! R-260: without an owner file the system is in the backtest stage.

use pt_gate::owner::{load_owner_file, Stage};

#[test]
fn missing_owner_file_means_backtest_stage_with_nothing_granted() {
    // R-260
    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("owner.toml");
    assert!(!path.exists());

    let owner = load_owner_file(&path, repo.path()).expect("a missing file is not an error");

    assert_eq!(owner.stage, Stage::Backtest);
    assert!(owner.strategy_allowlist.is_empty());
    assert_eq!(owner.account_flat_confirmed_on, None);
    assert!(owner.first_order.is_none());
    assert_eq!(owner.validation_signoff, None);
    assert_eq!(owner.alert_webhook_url, None);
}

#[test]
fn stages_are_ordered_backtest_to_paper() {
    // R-260 (Stage type shape; ordering is also required by R-263)
    assert!(Stage::Backtest < Stage::Ingest);
    assert!(Stage::Ingest < Stage::DryRun);
    assert!(Stage::DryRun < Stage::FirstOrder);
    assert!(Stage::FirstOrder < Stage::Paper);
}
