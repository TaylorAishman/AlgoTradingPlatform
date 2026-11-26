//! R-427: positions and cash at a date come from the same fold as today's.

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

fn day(n: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, n).unwrap()
}

fn entry(symbol: Option<&str>, qty: i64, cash: &str, when: NaiveDate) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: when,
        kind: if symbol.is_some() { "fill" } else { "opening" }.to_string(),
        symbol: symbol.map(sym),
        qty,
        price: None,
        cash_delta: d(cash),
        ref_id: None,
    }
}

fn loaded() -> Store {
    let store = Store::open_in_memory().unwrap();
    for e in [
        entry(None, 0, "100000", day(2)),
        entry(Some("SPY"), 10, "-5000", day(3)),
        entry(Some("SPY"), -15, "7650", day(4)),
        entry(Some("QQQ"), 5, "-2000", day(5)),
        entry(Some("QQQ"), -5, "2000", day(5)),
    ] {
        store.append_ledger(&e).unwrap();
    }
    store
}

// R-427
#[test]
fn positions_today_equal_positions_at_the_far_future() {
    let store = loaded();
    assert_eq!(
        store.positions(Book::Paper).unwrap(),
        store.positions_at(Book::Paper, NaiveDate::MAX).unwrap()
    );
}

// R-427
#[test]
fn cash_today_equals_cash_at_the_far_future() {
    let store = loaded();
    assert_eq!(
        store.cash(Book::Paper).unwrap(),
        store.cash_at(Book::Paper, NaiveDate::MAX).unwrap()
    );
}

// R-427
#[test]
fn positions_at_a_date_are_the_net_of_entries_to_that_date() {
    let store = loaded();
    let mut want = Positions::new();
    want.insert(sym("SPY"), 10);
    assert_eq!(store.positions_at(Book::Paper, day(3)).unwrap(), want);
}

// R-427
#[test]
fn cash_at_a_date_is_the_sum_of_deltas_to_that_date() {
    let store = loaded();
    assert_eq!(store.cash_at(Book::Paper, day(3)).unwrap(), d("95000"));
}

// R-427
#[test]
fn flat_symbols_are_absent_from_positions() {
    let store = loaded();
    let today = store.positions(Book::Paper).unwrap();
    assert!(!today.contains_key(&sym("QQQ")));
    assert_eq!(today.get(&sym("SPY")), Some(&-5));
    let at = store.positions_at(Book::Paper, day(5)).unwrap();
    assert!(!at.contains_key(&sym("QQQ")));
}
