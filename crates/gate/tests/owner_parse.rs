//! R-261: the owner file must parse strictly.

use std::path::Path;

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::owner::{load_owner_file, OwnerFile, Stage};

fn load(contents: &str) -> Result<OwnerFile, GateError> {
    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("owner.toml");
    std::fs::write(&path, contents).unwrap();
    load_owner_file(Path::new(&path), repo.path())
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn full_owner_file_loads_with_its_values() {
    // R-261
    let owner = load(
        r#"
stage = "dry_run"
strategy_allowlist = ["tsmom-v1"]
account_flat_confirmed_on = 2026-12-01

[first_order]
symbol = "SPY"
side = "buy"
max_qty = 1
valid_on = 2026-12-02
"#,
    )
    .expect("a valid owner file loads");

    assert_eq!(owner.stage, Stage::DryRun);
    assert_eq!(owner.strategy_allowlist, vec!["tsmom-v1".to_string()]);
    assert_eq!(owner.account_flat_confirmed_on, Some(date(2026, 12, 1)));
    let first = owner.first_order.expect("first_order table is loaded");
    assert_eq!(first.symbol, "SPY");
    assert_eq!(first.side, "buy");
    assert_eq!(first.max_qty, 1);
    assert_eq!(first.valid_on, date(2026, 12, 2));
    assert_eq!(owner.validation_signoff, None);
    assert_eq!(owner.alert_webhook_url, None);
}

#[test]
fn optional_text_keys_load() {
    // R-261 (every key of OwnerFile is accepted)
    let owner = load(
        r#"
stage = "paper"
validation_signoff = "signed 2026-12-05"
alert_webhook_url = "https://example.invalid/hook"
"#,
    )
    .unwrap();

    assert_eq!(owner.stage, Stage::Paper);
    assert_eq!(
        owner.validation_signoff.as_deref(),
        Some("signed 2026-12-05")
    );
    assert_eq!(
        owner.alert_webhook_url.as_deref(),
        Some("https://example.invalid/hook")
    );
}

#[test]
fn every_stage_name_is_accepted() {
    // R-261
    for (name, stage) in [
        ("backtest", Stage::Backtest),
        ("ingest", Stage::Ingest),
        ("dry_run", Stage::DryRun),
        ("first_order", Stage::FirstOrder),
        ("paper", Stage::Paper),
    ] {
        let owner = load(&format!("stage = \"{name}\"\n")).unwrap();
        assert_eq!(owner.stage, stage, "stage {name}");
    }
}

#[test]
fn stage_live_is_a_parse_error() {
    // R-261
    let err = load("stage = \"live\"\n").unwrap_err();
    assert!(matches!(err, GateError::Parse(_)), "got {err:?}");
}

#[test]
fn unknown_top_level_key_is_a_parse_error() {
    // R-261
    let err = load("stage = \"paper\"\nfoo = 1\n").unwrap_err();
    assert!(matches!(err, GateError::Parse(_)), "got {err:?}");
}

#[test]
fn unknown_key_in_first_order_table_is_a_parse_error() {
    // R-261
    let err = load(
        r#"
stage = "first_order"

[first_order]
symbol = "SPY"
side = "buy"
max_qty = 1
valid_on = 2026-12-02
foo = 1
"#,
    )
    .unwrap_err();
    assert!(matches!(err, GateError::Parse(_)), "got {err:?}");
}

#[test]
fn malformed_toml_is_a_parse_error() {
    // R-261
    let err = load("stage = = paper").unwrap_err();
    assert!(matches!(err, GateError::Parse(_)), "got {err:?}");
}
