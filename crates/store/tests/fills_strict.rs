//! R-420: recording a fill ignores a repeated activity id and nothing else.

use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::trading::FillRow;
use rust_decimal::Decimal;

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, 14, 30, 0).unwrap()
}

fn fill(activity: &str, broker_order: &str, qty: u64) -> FillRow {
    FillRow {
        activity_id: activity.to_string(),
        broker_order_id: broker_order.to_string(),
        symbol: Symbol::new("SPY").unwrap(),
        side: Side::Buy,
        qty,
        price: Decimal::from_str("500.10").unwrap(),
        at: at(),
    }
}

// R-420
#[test]
fn repeated_activity_id_returns_false_and_keeps_only_the_first_row() {
    let store = Store::open_in_memory().unwrap();
    let first = fill("a1", "b1", 10);
    let other = fill("a1", "b2", 3);
    assert!(store.insert_fill(&first).unwrap());
    assert!(!store.insert_fill(&other).unwrap());
    assert_eq!(store.fills().unwrap(), vec![first]);
}

// R-420
#[test]
fn collision_on_another_unique_index_is_an_error_not_ok_false() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let store = Store::open(&path).unwrap();
    let first = fill("a1", "b1", 10);
    assert!(store.insert_fill(&first).unwrap());

    let other = rusqlite::Connection::open(&path).unwrap();
    other
        .execute_batch("CREATE UNIQUE INDEX t ON fills (broker_order_id, at)")
        .unwrap();
    drop(other);

    let colliding = fill("a2", "b1", 5);
    assert!(store.insert_fill(&colliding).is_err());
    assert_eq!(store.fills().unwrap(), vec![first]);
}
