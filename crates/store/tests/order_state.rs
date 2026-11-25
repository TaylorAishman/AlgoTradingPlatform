//! R-062: order state changes obey the transition table and are logged.

use std::path::Path;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(secs, 0).unwrap()
}

fn order(id: &str, state: OrderState) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: Symbol::new("SPY").unwrap(),
        side: Side::Buy,
        qty: 10,
        leg: 0,
        state,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from_str("500.25").unwrap(),
        trade_date: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
    }
}

fn event_count(db: &Path, id: &str) -> i64 {
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM order_events WHERE client_order_id = ?1",
        [id],
        |r| r.get(0),
    )
    .unwrap()
}

// R-062
#[test]
fn legal_transition_updates_state_and_broker_order_id() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_order(&order("c1", OrderState::Intent))
        .unwrap();
    store
        .set_order_state(
            "c1",
            OrderState::Submitted,
            0,
            Some("b1"),
            at(1_796_000_000),
        )
        .unwrap();
    let got = store.order("c1").unwrap().unwrap();
    assert_eq!(got.state, OrderState::Submitted);
    assert_eq!(got.broker_order_id.as_deref(), Some("b1"));
}

// R-062
#[test]
fn partial_then_full_fill_records_filled_qty() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_order(&order("c1", OrderState::Submitted))
        .unwrap();
    store
        .set_order_state("c1", OrderState::PartiallyFilled, 4, None, at(1))
        .unwrap();
    assert_eq!(store.order("c1").unwrap().unwrap().filled_qty, 4);
    store
        .set_order_state("c1", OrderState::PartiallyFilled, 7, None, at(2))
        .unwrap();
    store
        .set_order_state("c1", OrderState::Filled, 10, None, at(3))
        .unwrap();
    let got = store.order("c1").unwrap().unwrap();
    assert_eq!(got.state, OrderState::Filled);
    assert_eq!(got.filled_qty, 10);
}

// R-062
#[test]
fn illegal_transition_is_rejected_and_row_is_unchanged() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_order(&order("c1", OrderState::Intent))
        .unwrap();
    store
        .set_order_state("c1", OrderState::Submitted, 0, Some("b1"), at(1))
        .unwrap();
    let before = store.order("c1").unwrap();
    let err = store
        .set_order_state("c1", OrderState::Intent, 3, Some("other"), at(2))
        .unwrap_err();
    assert!(
        matches!(err, StoreError::IllegalTransition { .. }),
        "got {err:?}"
    );
    assert_eq!(store.order("c1").unwrap(), before);
}

// R-062
#[test]
fn terminal_state_cannot_change() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_order(&order("c1", OrderState::Filled))
        .unwrap();
    let err = store
        .set_order_state("c1", OrderState::Canceled, 0, None, at(1))
        .unwrap_err();
    assert!(
        matches!(err, StoreError::IllegalTransition { .. }),
        "got {err:?}"
    );
    assert_eq!(
        store.order("c1").unwrap().unwrap().state,
        OrderState::Filled
    );
}

// R-062
#[test]
fn unknown_order_is_not_found() {
    let store = Store::open_in_memory().unwrap();
    let err = store
        .set_order_state("nope", OrderState::Submitted, 0, None, at(1))
        .unwrap_err();
    assert!(matches!(err, StoreError::NotFound(_)), "got {err:?}");
}

// R-062
#[test]
fn event_log_has_one_row_per_successful_change_including_creation() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let store = Store::open(&db).unwrap();
    store
        .insert_order(&order("c1", OrderState::Intent))
        .unwrap();
    store
        .insert_order(&order("other", OrderState::Intent))
        .unwrap();
    assert_eq!(event_count(&db, "c1"), 1);
    store
        .set_order_state("c1", OrderState::Submitted, 0, Some("b1"), at(1))
        .unwrap();
    assert_eq!(event_count(&db, "c1"), 2);
    store
        .set_order_state("c1", OrderState::Intent, 0, None, at(2))
        .unwrap_err();
    assert_eq!(event_count(&db, "c1"), 2);
    assert_eq!(event_count(&db, "other"), 1);
}

// R-062
#[test]
fn event_row_records_from_to_filled_qty_and_time() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let store = Store::open(&db).unwrap();
    store
        .insert_order(&order("c1", OrderState::Submitted))
        .unwrap();
    store
        .set_order_state(
            "c1",
            OrderState::PartiallyFilled,
            4,
            Some("b1"),
            at(1_796_000_000),
        )
        .unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    let (from, to, filled, broker): (Option<String>, String, i64, Option<String>) = conn
        .query_row(
            "SELECT from_state, to_state, filled_qty, broker_order_id FROM order_events \
             WHERE client_order_id = 'c1' ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(from.as_deref(), Some("submitted"));
    assert_eq!(to, "partially_filled");
    assert_eq!(filled, 4);
    assert_eq!(broker.as_deref(), Some("b1"));
}
