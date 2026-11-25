//! R-057: bars, fills and ledger entries are append-only at the database level.

use std::str::FromStr;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::market::BarRow;
use pt_store::trading::{Book, FillRow, LedgerEntry};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn t() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, 14, 30, 0).unwrap()
}

fn spy() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()
}

fn bar_row() -> BarRow {
    let p = |s: &str| Price::new(d(s)).unwrap();
    BarRow {
        bar: Bar {
            symbol: spy(),
            date: day(),
            open: p("99.5"),
            high: p("101.25"),
            low: p("98.75"),
            close: p("100"),
            volume: 1_000,
        },
        adjustment: Adjustment::Raw,
        feed: "sip".to_string(),
    }
}

fn fill_row() -> FillRow {
    FillRow {
        activity_id: "a1".to_string(),
        broker_order_id: "b1".to_string(),
        symbol: spy(),
        side: Side::Buy,
        qty: 10,
        price: d("500.12"),
        at: t(),
    }
}

fn ledger_entry() -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
        kind: "opening".to_string(),
        symbol: None,
        qty: 0,
        price: None,
        cash_delta: d("100000"),
        ref_id: None,
    }
}

fn seeded() -> Store {
    let s = Store::open_in_memory().unwrap();
    s.upsert_bars(&[bar_row()], t()).unwrap();
    assert!(s.insert_fill(&fill_row()).unwrap());
    s.append_ledger(&ledger_entry()).unwrap();
    s
}

fn assert_sql_err<T: std::fmt::Debug>(r: Result<T, StoreError>) {
    assert!(matches!(r, Err(StoreError::Sql(_))), "got {r:?}");
}

fn assert_unchanged(s: &Store) {
    let bars = s
        .bars_asof(&spy(), Adjustment::Raw, "sip", day(), day(), None)
        .unwrap();
    assert_eq!(bars, vec![bar_row().bar]);
    assert_eq!(s.fills().unwrap(), vec![fill_row()]);
    assert_eq!(s.ledger(Book::Paper).unwrap(), vec![ledger_entry()]);
}

#[test]
fn update_of_a_bar_is_refused() {
    // R-057
    let s = seeded();
    assert_sql_err(s.execute_raw("UPDATE bars SET close = '1'"));
    assert_unchanged(&s);
}

#[test]
fn delete_of_a_bar_is_refused() {
    // R-057
    let s = seeded();
    assert_sql_err(s.execute_raw("DELETE FROM bars"));
    assert_unchanged(&s);
}

#[test]
fn update_of_a_fill_is_refused() {
    // R-057
    let s = seeded();
    assert_sql_err(s.execute_raw("UPDATE fills SET qty = 99"));
    assert_unchanged(&s);
}

#[test]
fn delete_of_a_fill_is_refused() {
    // R-057
    let s = seeded();
    assert_sql_err(s.execute_raw("DELETE FROM fills"));
    assert_unchanged(&s);
}

#[test]
fn update_of_a_ledger_entry_is_refused() {
    // R-057
    let s = seeded();
    assert_sql_err(s.execute_raw("UPDATE ledger_entries SET qty = 5"));
    assert_unchanged(&s);
}

#[test]
fn delete_of_a_ledger_entry_is_refused() {
    // R-057
    let s = seeded();
    assert_sql_err(s.execute_raw("DELETE FROM ledger_entries"));
    assert_unchanged(&s);
}

#[test]
fn appending_still_works() {
    // R-057
    let s = seeded();
    s.append_ledger(&ledger_entry()).unwrap();
    assert_eq!(s.ledger(Book::Paper).unwrap().len(), 2);
}
