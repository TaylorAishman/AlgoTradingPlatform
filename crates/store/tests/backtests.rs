//! R-072: backtest equity must round-trip exactly.

use chrono::{NaiveDate, TimeZone, Utc};
use pt_store::db::Store;
use pt_store::error::StoreError;
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn day(y: i32, m: u32, dd: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, dd).unwrap()
}

#[test]
fn equity_round_trips_exactly_in_date_order() {
    // R-072
    let store = Store::open_in_memory().unwrap();
    let equity = vec![
        (day(2024, 1, 2), d("100000")),
        (day(2024, 1, 3), d("100922.60")),
        (day(2024, 1, 4), d("101628.2602")),
    ];
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let id = store
        .save_backtest("tsmom-v1", "h1", "cfg1", &equity, at)
        .unwrap();
    let back = store.backtest_equity(id).unwrap();
    assert_eq!(back, equity);
    // Scale is preserved too, not only the numeric value.
    assert_eq!(back[1].1.to_string(), "100922.60");
}

#[test]
fn equity_is_returned_in_date_order_even_if_saved_out_of_order() {
    // R-072
    let store = Store::open_in_memory().unwrap();
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let equity = vec![
        (day(2024, 1, 4), d("3")),
        (day(2024, 1, 2), d("1")),
        (day(2024, 1, 3), d("2")),
    ];
    let id = store.save_backtest("f", "h", "c", &equity, at).unwrap();
    let dates: Vec<NaiveDate> = store
        .backtest_equity(id)
        .unwrap()
        .into_iter()
        .map(|p| p.0)
        .collect();
    assert_eq!(
        dates,
        vec![day(2024, 1, 2), day(2024, 1, 3), day(2024, 1, 4)]
    );
}

#[test]
fn separate_backtests_get_distinct_ids_and_curves() {
    // R-072
    let store = Store::open_in_memory().unwrap();
    let at = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    let a = vec![(day(2024, 1, 2), d("10"))];
    let b = vec![(day(2024, 1, 2), d("20")), (day(2024, 1, 3), d("21"))];
    let ida = store.save_backtest("f", "h1", "c", &a, at).unwrap();
    let idb = store.save_backtest("f", "h2", "c", &b, at).unwrap();
    assert_ne!(ida, idb);
    assert_eq!(store.backtest_equity(ida).unwrap(), a);
    assert_eq!(store.backtest_equity(idb).unwrap(), b);
}

#[test]
fn unknown_backtest_is_not_found() {
    // R-072
    let store = Store::open_in_memory().unwrap();
    assert!(matches!(
        store.backtest_equity(999),
        Err(StoreError::NotFound(_))
    ));
}
