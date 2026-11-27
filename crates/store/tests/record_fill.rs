//! R-425: a fill and its ledger entry are written together or not at all.

use std::str::FromStr;

use chrono::{NaiveDate, TimeZone, Utc};
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::trading::{Book, FillRow, LedgerEntry};
use rust_decimal::Decimal;

fn fill(activity: &str) -> FillRow {
    FillRow {
        activity_id: activity.to_string(),
        broker_order_id: "b1".to_string(),
        symbol: Symbol::new("SPY").unwrap(),
        side: Side::Buy,
        qty: 10,
        price: Decimal::from_str("500.25").unwrap(),
        at: Utc.with_ymd_and_hms(2026, 12, 1, 15, 0, 0).unwrap(),
    }
}

fn entry(ref_id: &str) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
        kind: "fill".to_string(),
        symbol: Some(Symbol::new("SPY").unwrap()),
        qty: 10,
        price: Some(Decimal::from_str("500.25").unwrap()),
        cash_delta: Decimal::from_str("-5002.50").unwrap(),
        ref_id: Some(ref_id.to_string()),
    }
}

// R-425
#[test]
fn record_fill_writes_the_fill_and_the_entry() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.record_fill(&fill("a1"), &entry("a1")).unwrap());
    assert_eq!(store.fills().unwrap(), vec![fill("a1")]);
    assert_eq!(store.ledger(Book::Paper).unwrap(), vec![entry("a1")]);
}

// R-425
#[test]
fn a_repeated_activity_id_returns_false_and_adds_neither() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.record_fill(&fill("a1"), &entry("a1")).unwrap());
    assert!(!store.record_fill(&fill("a1"), &entry("a1-again")).unwrap());
    assert_eq!(store.fills().unwrap().len(), 1);
    assert_eq!(store.ledger(Book::Paper).unwrap(), vec![entry("a1")]);
}

// R-425
#[test]
fn a_failing_ledger_entry_leaves_no_fill_behind() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let store = Store::open(&db).unwrap();
    store.append_ledger(&entry("dup")).unwrap();

    let other = rusqlite::Connection::open(&db).unwrap();
    other
        .execute("CREATE UNIQUE INDEX t ON ledger_entries (ref_id)", [])
        .unwrap();

    let result = store.record_fill(&fill("new"), &entry("dup"));
    assert!(result.is_err(), "got {result:?}");
    assert!(store.fills().unwrap().is_empty());
    assert_eq!(store.ledger(Book::Paper).unwrap().len(), 1);
}
