//! D41: a stored backtest keeps its starting capital and where it came from.

use chrono::{NaiveDate, TimeZone, Utc};
use pt_store::db::Store;
use pt_store::error::StoreError;
use rust_decimal::Decimal;

fn equity() -> Vec<(NaiveDate, Decimal)> {
    vec![
        (
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            Decimal::from(250000),
        ),
        (
            NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
            Decimal::new(25012550, 2),
        ),
    ]
}

#[test]
fn capital_and_source_round_trip_with_the_equity_curve() {
    // R-430
    let store = Store::open_in_memory().unwrap();
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let eq = equity();
    let id = store
        .save_backtest_with_capital("f", "h", "c", Decimal::from(250000), "flag", &eq, at)
        .unwrap();
    assert_eq!(
        store.backtest_capital(id).unwrap(),
        Some((Decimal::from(250000), "flag".to_string()))
    );
    assert_eq!(store.backtest_equity(id).unwrap(), eq);
    assert_eq!(store.backtest_family(id).unwrap(), "f");
}

#[test]
fn run_saved_without_capital_reads_none() {
    // R-430
    let store = Store::open_in_memory().unwrap();
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let id = store.save_backtest("f", "h", "c", &equity(), at).unwrap();
    assert_eq!(store.backtest_capital(id).unwrap(), None);
}

#[test]
fn capital_of_an_unsaved_backtest_is_not_found() {
    // R-430
    let store = Store::open_in_memory().unwrap();
    assert!(matches!(
        store.backtest_capital(999),
        Err(StoreError::NotFound(_))
    ));
}
