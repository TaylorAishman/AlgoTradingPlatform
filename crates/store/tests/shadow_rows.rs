//! R-067: the store keeps shadow comparison rows and slippage rows.

use chrono::NaiveDate;
use pt_store::db::Store;
use pt_store::shadow::{ShadowDay, SlippageRow};
use rust_decimal::Decimal;
use std::str::FromStr;

fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn day(d: &str, paper: f64, shadow: f64) -> ShadowDay {
    ShadowDay {
        date: date(d),
        paper_return: paper,
        shadow_return: shadow,
        diff: paper - shadow,
    }
}

fn slip(id: &str, bps: f64) -> SlippageRow {
    SlippageRow {
        client_order_id: id.to_string(),
        reference_price: dec("412.3500"),
        avg_fill_price: dec("412.4100"),
        qty: 25,
        bps,
    }
}

#[test]
fn put_shadow_day_then_shadow_days_returns_the_row() {
    // R-067
    let store = Store::open_in_memory().unwrap();
    assert!(store.shadow_days().unwrap().is_empty());
    store
        .put_shadow_day(&day("2026-12-01", 0.0125, 0.0120))
        .unwrap();
    let rows = store.shadow_days().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].date, date("2026-12-01"));
    assert_eq!(rows[0].paper_return, 0.0125);
    assert_eq!(rows[0].shadow_return, 0.0120);
    assert_eq!(rows[0].diff, 0.0125 - 0.0120);
}

#[test]
fn putting_the_same_shadow_date_again_replaces_it() {
    // R-067
    let store = Store::open_in_memory().unwrap();
    store
        .put_shadow_day(&day("2026-12-01", 0.01, 0.02))
        .unwrap();
    store
        .put_shadow_day(&day("2026-12-01", 0.03, 0.01))
        .unwrap();
    let rows = store.shadow_days().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].paper_return, 0.03);
    assert_eq!(rows[0].shadow_return, 0.01);
    assert_eq!(rows[0].diff, 0.03 - 0.01);
}

#[test]
fn shadow_days_are_returned_in_date_order() {
    // R-067
    let store = Store::open_in_memory().unwrap();
    store.put_shadow_day(&day("2026-12-02", 0.0, 0.0)).unwrap();
    store.put_shadow_day(&day("2026-12-01", 0.0, 0.0)).unwrap();
    let dates: Vec<NaiveDate> = store
        .shadow_days()
        .unwrap()
        .iter()
        .map(|d| d.date)
        .collect();
    assert_eq!(dates, vec![date("2026-12-01"), date("2026-12-02")]);
}

#[test]
fn put_slippage_then_slippage_rows_returns_the_row_with_its_bps() {
    // R-067
    let store = Store::open_in_memory().unwrap();
    assert!(store.slippage_rows().unwrap().is_empty());
    store.put_slippage(&slip("co-1", 1.455)).unwrap();
    let rows = store.slippage_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].client_order_id, "co-1");
    assert_eq!(rows[0].reference_price, dec("412.3500"));
    assert_eq!(rows[0].avg_fill_price, dec("412.4100"));
    assert_eq!(rows[0].qty, 25);
    assert_eq!(rows[0].bps, 1.455);
}

#[test]
fn putting_the_same_client_order_id_again_replaces_the_slippage_row() {
    // R-067
    let store = Store::open_in_memory().unwrap();
    store.put_slippage(&slip("co-1", 1.0)).unwrap();
    store.put_slippage(&slip("co-2", 2.0)).unwrap();
    store.put_slippage(&slip("co-1", -3.5)).unwrap();
    let rows = store.slippage_rows().unwrap();
    assert_eq!(rows.len(), 2);
    let first = rows.iter().find(|r| r.client_order_id == "co-1").unwrap();
    assert_eq!(first.bps, -3.5);
}
