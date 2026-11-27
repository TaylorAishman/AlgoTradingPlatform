//! R-271: the committed example owner file must load and grant nothing.

use std::path::PathBuf;

use chrono::NaiveDate;
use pt_gate::owner::{load_owner_file, Stage};

fn example_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/owner.example.toml")
}

#[test]
fn example_file_copied_outside_the_repo_loads_and_grants_nothing() {
    // R-271
    let text = std::fs::read_to_string(example_path()).expect("config/owner.example.toml exists");
    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let copy = outside.path().join("owner.toml");
    std::fs::write(&copy, text).unwrap();

    let owner = load_owner_file(&copy, repo.path()).expect("the example loads");

    assert_eq!(owner.stage, Stage::Backtest);
    assert!(owner.strategy_allowlist.is_empty());
    assert_eq!(owner.account_flat_confirmed_on, None);
    assert_eq!(owner.first_order, None);
    assert_eq!(owner.validation_signoff, None);
    assert_eq!(owner.alert_webhook_url, None);
}

#[test]
fn example_file_shows_every_key_as_a_comment() {
    // R-271
    let text = std::fs::read_to_string(example_path()).expect("config/owner.example.toml exists");
    let comments: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('#'))
        .collect();
    let has = |needle: &str| comments.iter().any(|l| l.contains(needle));

    for key in [
        "stage",
        "strategy_allowlist",
        "account_flat_confirmed_on",
        "validation_signoff",
        "alert_webhook_url",
        "symbol",
        "side",
        "qty",
        "max_qty",
        "valid_on",
    ] {
        assert!(has(&format!("{key} =")), "no commented example of `{key}`");
    }
    assert!(has("[first_order]"), "no commented [first_order] table");
}

#[test]
fn example_file_with_every_example_uncommented_loads_with_the_shown_values() {
    // R-271
    let text = std::fs::read_to_string(example_path()).expect("config/owner.example.toml exists");
    let mut kept = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let rest = rest.trim_start();
            let is_example = rest.starts_with("[first_order]")
                || [
                    "stage",
                    "strategy_allowlist",
                    "account_flat_confirmed_on",
                    "validation_signoff",
                    "alert_webhook_url",
                    "symbol",
                    "side",
                    "qty",
                    "max_qty",
                    "valid_on",
                ]
                .iter()
                .any(|k| rest.starts_with(&format!("{k} =")));
            if is_example {
                kept.push(rest.to_string());
            }
        } else if trimmed.starts_with("stage") {
            // Replaced by the commented example `stage` line.
            continue;
        } else {
            kept.push(line.to_string());
        }
    }
    let uncommented = kept.join("\n");

    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let copy = outside.path().join("owner.toml");
    std::fs::write(&copy, &uncommented).unwrap();

    let owner = load_owner_file(&copy, repo.path())
        .unwrap_or_else(|e| panic!("uncommented example must load: {e:?}\n{uncommented}"));

    assert_eq!(owner.stage, Stage::Paper);
    assert_eq!(
        owner.strategy_allowlist,
        vec!["example_strategy".to_string()]
    );
    assert_eq!(
        owner.account_flat_confirmed_on,
        NaiveDate::from_ymd_opt(2026, 12, 1)
    );
    assert!(owner.validation_signoff.is_some());
    assert_eq!(
        owner.alert_webhook_url.as_deref(),
        Some("https://example.invalid/webhook")
    );
    let fo = owner.first_order.expect("first_order example");
    assert_eq!(fo.symbol, "SPY");
    assert_eq!(fo.side, "buy");
    assert_eq!(fo.qty, 1);
    assert_eq!(fo.max_qty, 1);
    assert_eq!(fo.valid_on, NaiveDate::from_ymd_opt(2026, 12, 2).unwrap());
}
