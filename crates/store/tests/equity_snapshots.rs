//! R-066: equity snapshots are replaceable but keep when first recorded.

use std::str::FromStr;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_store::db::Store;
use pt_store::trading::{Book, EquitySnapshot};
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn date(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 2, hour, 0, 0).unwrap()
}

fn snap(book: Book, when: NaiveDate, equity: &str, recorded_at: DateTime<Utc>) -> EquitySnapshot {
    EquitySnapshot {
        book,
        date: when,
        cash: d("40000"),
        positions_value: d(equity) - d("40000"),
        equity: d(equity),
        recorded_at,
    }
}

// R-066
#[test]
fn replacing_a_snapshot_keeps_first_recorded_at_and_new_values() {
    let store = Store::open_in_memory().unwrap();
    let (t1, t2) = (at(10), at(11));
    store
        .put_equity_snapshot(&snap(Book::Paper, date(12, 1), "100000", t1))
        .unwrap();
    store
        .put_equity_snapshot(&snap(Book::Paper, date(12, 1), "100500", t2))
        .unwrap();
    let rows = store.equity_series(Book::Paper).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].equity, d("100500"));
    assert_eq!(rows[0].positions_value, d("60500"));
    assert_eq!(rows[0].recorded_at, t1);
}

// R-066
#[test]
fn series_is_per_book_and_in_date_order() {
    let store = Store::open_in_memory().unwrap();
    store
        .put_equity_snapshot(&snap(Book::Paper, date(12, 2), "101000", at(10)))
        .unwrap();
    store
        .put_equity_snapshot(&snap(Book::Paper, date(12, 1), "100000", at(10)))
        .unwrap();
    store
        .put_equity_snapshot(&snap(Book::Shadow, date(12, 1), "99000", at(10)))
        .unwrap();
    let paper = store.equity_series(Book::Paper).unwrap();
    assert_eq!(paper.len(), 2);
    assert_eq!(paper[0].date, date(12, 1));
    assert_eq!(paper[1].date, date(12, 2));
    let shadow = store.equity_series(Book::Shadow).unwrap();
    assert_eq!(shadow.len(), 1);
    assert_eq!(shadow[0].equity, d("99000"));
}

// R-066
#[test]
fn empty_book_has_empty_series() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.equity_series(Book::Paper).unwrap().is_empty());
}
