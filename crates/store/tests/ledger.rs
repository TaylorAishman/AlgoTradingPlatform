//! R-064, R-073: positions and cash derive from the ledger; entries list in insertion order.

use std::str::FromStr;

use chrono::NaiveDate;
use pt_core::types::{Positions, Symbol};
use pt_store::db::Store;
use pt_store::trading::{Book, LedgerEntry};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, day).unwrap()
}

fn opening(book: Book) -> LedgerEntry {
    LedgerEntry {
        book,
        date: NaiveDate::from_ymd_opt(2026, 11, 30).unwrap(),
        kind: "opening".to_string(),
        symbol: None,
        qty: 0,
        price: None,
        cash_delta: d("100000"),
        ref_id: None,
    }
}

fn fill(symbol: &str, qty: i64, price: &str, cash: &str, day: u32, r: &str) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: date(day),
        kind: "fill".to_string(),
        symbol: Some(sym(symbol)),
        qty,
        price: Some(d(price)),
        cash_delta: d(cash),
        ref_id: Some(r.to_string()),
    }
}

fn r064_entries() -> Vec<LedgerEntry> {
    vec![
        opening(Book::Paper),
        fill("SPY", 10, "500", "-5000", 1, "a1"),
        fill("SPY", -15, "510", "7650", 2, "a2"),
        fill("QQQ", 5, "400.5", "-2002.5", 3, "a3"),
        fill("QQQ", -5, "400.5", "2002.5", 3, "a4"),
    ]
}

fn loaded() -> (Store, Vec<LedgerEntry>) {
    let store = Store::open_in_memory().unwrap();
    let entries = r064_entries();
    for e in &entries {
        store.append_ledger(e).unwrap();
    }
    (store, entries)
}

// R-064
#[test]
fn positions_are_derived_from_the_ledger_and_flat_symbols_are_absent() {
    let (store, _) = loaded();
    let mut want = Positions::new();
    want.insert(sym("SPY"), -5);
    assert_eq!(store.positions(Book::Paper).unwrap(), want);
}

// R-064
#[test]
fn cash_is_the_sum_of_cash_deltas() {
    let (store, _) = loaded();
    assert_eq!(store.cash(Book::Paper).unwrap(), d("102650"));
}

// R-064
#[test]
fn the_shadow_book_is_empty() {
    let (store, _) = loaded();
    assert!(store.positions(Book::Shadow).unwrap().is_empty());
    assert_eq!(store.cash(Book::Shadow).unwrap(), Decimal::ZERO);
}

// R-064
#[test]
fn append_ledger_returns_distinct_increasing_ids() {
    let store = Store::open_in_memory().unwrap();
    let a = store.append_ledger(&opening(Book::Paper)).unwrap();
    let b = store.append_ledger(&opening(Book::Shadow)).unwrap();
    assert!(b > a);
}

// R-073
#[test]
fn ledger_lists_entries_in_insertion_order_with_equal_fields() {
    let (store, entries) = loaded();
    assert_eq!(store.ledger(Book::Paper).unwrap(), entries);
}

// R-073
#[test]
fn ledger_is_empty_for_the_other_book() {
    let (store, _) = loaded();
    assert!(store.ledger(Book::Shadow).unwrap().is_empty());
}
