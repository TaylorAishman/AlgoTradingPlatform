//! R-423: an order's creation event is stamped at 00:00 UTC of its trade date.

use std::str::FromStr;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::trading::OrderRow;
use rusqlite::Connection;
use rust_decimal::Decimal;

fn order(id: &str, trade_date: NaiveDate) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: Symbol::new("SPY").unwrap(),
        side: Side::Buy,
        qty: 10,
        leg: 0,
        state: OrderState::Submitted,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from_str("500.25").unwrap(),
        trade_date,
    }
}

fn first_event_at(path: &std::path::Path, id: &str) -> DateTime<Utc> {
    let conn = Connection::open(path).unwrap();
    let text: String = conn
        .query_row(
            "SELECT at FROM order_events WHERE client_order_id = ?1 ORDER BY id LIMIT 1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    DateTime::parse_from_rfc3339(&text)
        .unwrap()
        .with_timezone(&Utc)
}

// R-423
#[test]
fn the_creation_event_is_at_midnight_utc_of_the_trade_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let store = Store::open(&path).unwrap();
    store
        .insert_order(&order("c1", NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()))
        .unwrap();
    assert_eq!(
        first_event_at(&path, "c1"),
        Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap()
    );
}

// R-423
#[test]
fn another_trade_date_gives_another_creation_instant() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let store = Store::open(&path).unwrap();
    store
        .insert_order(&order("c2", NaiveDate::from_ymd_opt(2031, 12, 31).unwrap()))
        .unwrap();
    assert_eq!(
        first_event_at(&path, "c2"),
        Utc.with_ymd_and_hms(2031, 12, 31, 0, 0, 0).unwrap()
    );
}
