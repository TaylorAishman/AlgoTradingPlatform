//! R-060: at most one decision per strategy, signal date and mode.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::trading::{DecisionRow, Mode};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 30, 21, 5, 0).unwrap()
}

fn decision(strategy: &str, signal: NaiveDate, mode: Mode) -> DecisionRow {
    DecisionRow {
        id: 0,
        strategy_id: strategy.to_string(),
        signal_date: signal,
        trade_date: Some(date(2026, 12, 1)),
        mode,
        status: "planned".to_string(),
        inputs_json: r#"{"weights":{"SPY":"0.25"},"note":"in"}"#.to_string(),
        outputs_json: r#"{"orders":[{"symbol":"SPY","qty":10}]}"#.to_string(),
        created_at: at(),
    }
}

#[test]
fn second_insert_of_same_strategy_date_and_mode_is_duplicate() {
    let store = Store::open_in_memory().unwrap();
    let d = decision("tsmom-v1", date(2026, 11, 30), Mode::Paper);
    store.insert_decision(&d).unwrap();
    let err = store.insert_decision(&d).unwrap_err();
    assert!(matches!(err, StoreError::Duplicate(_)), "got {err:?}");
    assert_eq!(store.decisions(Mode::Paper).unwrap().len(), 1);
}

#[test]
fn same_strategy_and_date_with_other_mode_succeeds() {
    let store = Store::open_in_memory().unwrap();
    let paper = store
        .insert_decision(&decision("tsmom-v1", date(2026, 11, 30), Mode::Paper))
        .unwrap();
    let dry = store
        .insert_decision(&decision("tsmom-v1", date(2026, 11, 30), Mode::DryRun))
        .unwrap();
    assert_ne!(paper, dry);
}

#[test]
fn decision_for_returns_stored_row_with_identical_json() {
    let store = Store::open_in_memory().unwrap();
    let d = decision("tsmom-v1", date(2026, 11, 30), Mode::Paper);
    let id = store.insert_decision(&d).unwrap();
    let got = store
        .decision_for("tsmom-v1", date(2026, 11, 30), Mode::Paper)
        .unwrap()
        .expect("stored row");
    assert_eq!(got.id, id);
    assert_eq!(got.inputs_json, d.inputs_json);
    assert_eq!(got.outputs_json, d.outputs_json);
    assert_eq!(got, DecisionRow { id, ..d });
}

#[test]
fn decision_for_is_none_when_absent_or_other_mode() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_decision(&decision("tsmom-v1", date(2026, 11, 30), Mode::Paper))
        .unwrap();
    assert!(store
        .decision_for("tsmom-v1", date(2026, 11, 30), Mode::DryRun)
        .unwrap()
        .is_none());
    assert!(store
        .decision_for("tsmom-v1", date(2026, 12, 1), Mode::Paper)
        .unwrap()
        .is_none());
    assert!(store
        .decision_for("other", date(2026, 11, 30), Mode::Paper)
        .unwrap()
        .is_none());
}

#[test]
fn decisions_lists_only_the_mode_in_insertion_order() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_decision(&decision("tsmom-v1", date(2026, 11, 30), Mode::Paper))
        .unwrap();
    store
        .insert_decision(&decision("tsmom-v1", date(2026, 11, 30), Mode::DryRun))
        .unwrap();
    store
        .insert_decision(&decision("tsmom-v1", date(2026, 12, 1), Mode::Paper))
        .unwrap();
    let paper = store.decisions(Mode::Paper).unwrap();
    let dates: Vec<NaiveDate> = paper.iter().map(|d| d.signal_date).collect();
    assert_eq!(dates, vec![date(2026, 11, 30), date(2026, 12, 1)]);
    assert!(paper.iter().all(|d| d.mode == Mode::Paper));
    assert_eq!(store.decisions(Mode::DryRun).unwrap().len(), 1);
}
