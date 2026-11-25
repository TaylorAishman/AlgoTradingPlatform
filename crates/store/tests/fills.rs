//! R-063, R-073: fills are recorded once per activity id and listed in insertion order.

use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::trading::FillRow;
use rust_decimal::Decimal;

fn at(sec: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, 14, 30, sec).unwrap()
}

fn fill(activity: &str, symbol: &str, side: Side, qty: u64, price: &str, sec: u32) -> FillRow {
    FillRow {
        activity_id: activity.to_string(),
        broker_order_id: format!("broker-{activity}"),
        symbol: Symbol::new(symbol).unwrap(),
        side,
        qty,
        price: Decimal::from_str(price).unwrap(),
        at: at(sec),
    }
}

// R-063
#[test]
fn insert_fill_is_true_first_time_and_false_second_time() {
    let store = Store::open_in_memory().unwrap();
    let f = fill("a1", "SPY", Side::Buy, 10, "500.123456", 1);
    assert!(store.insert_fill(&f).unwrap());
    assert!(!store.insert_fill(&f).unwrap());
}

// R-063
#[test]
fn repeated_insert_leaves_one_row_equal_to_the_first() {
    let store = Store::open_in_memory().unwrap();
    let f = fill("a1", "SPY", Side::Buy, 10, "500.123456", 1);
    store.insert_fill(&f).unwrap();
    store.insert_fill(&f).unwrap();
    assert_eq!(store.fills().unwrap(), vec![f]);
}

// R-063
#[test]
fn repeated_activity_id_with_other_data_keeps_the_first_row() {
    let store = Store::open_in_memory().unwrap();
    let first = fill("a1", "SPY", Side::Buy, 10, "500.10", 1);
    let other = fill("a1", "QQQ", Side::Sell, 3, "400.00", 2);
    assert!(store.insert_fill(&first).unwrap());
    assert!(!store.insert_fill(&other).unwrap());
    assert_eq!(store.fills().unwrap(), vec![first]);
}

// R-073
#[test]
fn fills_are_listed_in_insertion_order_with_equal_fields() {
    let store = Store::open_in_memory().unwrap();
    // The later-sorting activity id goes first, so ordering by id would fail.
    let first = fill("z9", "SPY", Side::Buy, 10, "500.25", 5);
    let second = fill("a1", "QQQ", Side::Sell, 4, "399.999999", 1);
    assert!(store.insert_fill(&first).unwrap());
    assert!(store.insert_fill(&second).unwrap());
    assert_eq!(store.fills().unwrap(), vec![first, second]);
}

// R-073
#[test]
fn fills_is_empty_on_a_new_store() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.fills().unwrap().is_empty());
}
