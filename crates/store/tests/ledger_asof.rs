//! R-065: positions and cash are readable as of a date.

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

fn date(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

fn entry(kind: &str, symbol: Option<&str>, qty: i64, cash: &str, when: NaiveDate) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: when,
        kind: kind.to_string(),
        symbol: symbol.map(sym),
        qty,
        price: None,
        cash_delta: d(cash),
        ref_id: None,
    }
}

fn loaded() -> Store {
    let store = Store::open_in_memory().unwrap();
    store
        .append_ledger(&entry("opening", None, 0, "100000", date(11, 30)))
        .unwrap();
    store
        .append_ledger(&entry("fill", Some("SPY"), 10, "-5000", date(12, 1)))
        .unwrap();
    store
        .append_ledger(&entry("fill", Some("SPY"), -4, "2100", date(12, 2)))
        .unwrap();
    store
}

// R-065
#[test]
fn positions_at_includes_entries_dated_on_that_day() {
    let store = loaded();
    let mut want = Positions::new();
    want.insert(sym("SPY"), 10);
    assert_eq!(store.positions_at(Book::Paper, date(12, 1)).unwrap(), want);
}

// R-065
#[test]
fn cash_at_includes_entries_dated_on_that_day() {
    let store = loaded();
    assert_eq!(store.cash_at(Book::Paper, date(12, 1)).unwrap(), d("95000"));
}

// R-065
#[test]
fn positions_at_before_any_fill_is_empty() {
    let store = loaded();
    assert!(store
        .positions_at(Book::Paper, date(11, 30))
        .unwrap()
        .is_empty());
}

// R-065
#[test]
fn cash_at_the_opening_date_is_the_opening_balance() {
    let store = loaded();
    assert_eq!(
        store.cash_at(Book::Paper, date(11, 30)).unwrap(),
        d("100000")
    );
}

// R-065
#[test]
fn as_of_the_last_day_matches_the_current_view() {
    let store = loaded();
    let mut want = Positions::new();
    want.insert(sym("SPY"), 6);
    assert_eq!(store.positions_at(Book::Paper, date(12, 2)).unwrap(), want);
    assert_eq!(store.cash_at(Book::Paper, date(12, 2)).unwrap(), d("97100"));
}

// R-065
#[test]
fn as_of_ignores_the_other_book() {
    let store = loaded();
    assert!(store
        .positions_at(Book::Shadow, date(12, 2))
        .unwrap()
        .is_empty());
    assert_eq!(
        store.cash_at(Book::Shadow, date(12, 2)).unwrap(),
        Decimal::ZERO
    );
}
