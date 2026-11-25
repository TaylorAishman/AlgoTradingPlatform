//! P07 (R-331): a decision that cannot be reproduced is reported with what differs.

mod replay_fixture;

use pt_engine::decision::record_to_json;
use pt_shadow::error::ShadowError;
use pt_shadow::replay::replay_decision;
use pt_store::trading::{DecisionRow, Mode};

/// Stores a copy of the fixture row as a dry-run decision (so it does not clash with the
/// paper row) with the given inputs and outputs, and returns its id.
fn insert_variant(f: &replay_fixture::Fixture, inputs_json: String, outputs_json: String) -> i64 {
    let row = DecisionRow {
        id: 0,
        mode: Mode::DryRun,
        inputs_json,
        outputs_json,
        ..f.row.clone()
    };
    f.store.insert_decision(&row).unwrap()
}

#[test]
fn an_altered_weight_is_reported_as_a_weights_diff() {
    // R-331
    let f = replay_fixture::build();
    let mut v: serde_json::Value = serde_json::from_str(&f.row.outputs_json).unwrap();
    let original = v["weights"]["SPY"].clone();
    let altered = if original == "0.123457" {
        "0.234567"
    } else {
        "0.123457"
    };
    v["weights"]["SPY"] = serde_json::Value::from(altered);
    let id = insert_variant(&f, f.row.inputs_json.clone(), v.to_string());

    let result = replay_decision(&f.store, &f.cfg, &f.strategy, id).unwrap();
    assert!(!result.matches);
    assert_eq!(result.decision_id, id);
    assert!(
        result.diffs.iter().any(|d| d.contains("weights")),
        "diffs: {:?}",
        result.diffs
    );
}

#[test]
fn a_different_config_hash_is_reported_as_a_config_hash_diff() {
    // R-331
    let f = replay_fixture::build();
    let mut record = f.record.clone();
    record.config_hash = "0000-not-the-current-config".to_string();
    let id = insert_variant(&f, record_to_json(&record), f.row.outputs_json.clone());

    let result = replay_decision(&f.store, &f.cfg, &f.strategy, id).unwrap();
    assert!(!result.matches);
    assert!(
        result.diffs.iter().any(|d| d.contains("config_hash")),
        "diffs: {:?}",
        result.diffs
    );
}

#[test]
fn an_untouched_weight_does_not_add_a_weights_diff_to_a_config_hash_mismatch() {
    // R-331: the diffs name only what differs.
    let f = replay_fixture::build();
    let mut record = f.record.clone();
    record.config_hash = "0000-not-the-current-config".to_string();
    let id = insert_variant(&f, record_to_json(&record), f.row.outputs_json.clone());

    let result = replay_decision(&f.store, &f.cfg, &f.strategy, id).unwrap();
    assert!(
        !result.diffs.iter().any(|d| d.contains("weights")),
        "diffs: {:?}",
        result.diffs
    );
}

#[test]
fn an_unknown_decision_id_is_not_found() {
    // R-331
    let f = replay_fixture::build();
    let err = replay_decision(&f.store, &f.cfg, &f.strategy, f.decision_id + 1000).unwrap_err();
    assert!(matches!(err, ShadowError::NotFound(_)), "{err:?}");
}
