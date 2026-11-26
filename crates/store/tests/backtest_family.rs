//! D27: the family of a saved backtest is readable from its run row alone.

use chrono::{TimeZone, Utc};
use pt_store::db::Store;
use pt_store::error::StoreError;

#[test]
fn family_round_trips_for_two_saved_backtests() {
    // D27, R-391
    let store = Store::open_in_memory().unwrap();
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let first = store.save_backtest("tsmom-v1", "h1", "c", &[], at).unwrap();
    let second = store
        .save_backtest("bh-volscaled-v1", "h2", "c", &[], at)
        .unwrap();
    assert_eq!(store.backtest_family(first).unwrap(), "tsmom-v1");
    assert_eq!(store.backtest_family(second).unwrap(), "bh-volscaled-v1");
}

#[test]
fn family_is_readable_for_a_run_with_empty_equity() {
    // D27: the run row alone is enough; no equity rows are needed.
    let store = Store::open_in_memory().unwrap();
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let id = store.save_backtest("f", "h", "c", &[], at).unwrap();
    assert!(store.backtest_equity(id).unwrap().is_empty());
    assert_eq!(store.backtest_family(id).unwrap(), "f");
}

#[test]
fn unknown_backtest_family_is_not_found() {
    // D27
    let store = Store::open_in_memory().unwrap();
    assert!(matches!(
        store.backtest_family(999),
        Err(StoreError::NotFound(_))
    ));
}
