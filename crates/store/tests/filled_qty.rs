//! R-421, R-422: the filled quantity of an order never exceeds its quantity
//! and never falls.

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

fn order(id: &str) -> OrderRow {
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
        trade_date: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
    }
}

fn stored() -> Store {
    let store = Store::open_in_memory().unwrap();
    store.insert_order(&order("c1")).unwrap();
    store
}

// R-421
#[test]
fn filled_qty_above_order_qty_is_invalid_and_leaves_the_row_unchanged() {
    let store = stored();
    let before = store.order("c1").unwrap();
    let err = store
        .set_order_state("c1", OrderState::PartiallyFilled, 11, None, at(1))
        .unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)), "got {err:?}");
    assert_eq!(store.order("c1").unwrap(), before);
}

// R-421
#[test]
fn filled_qty_equal_to_order_qty_is_accepted() {
    let store = stored();
    store
        .set_order_state("c1", OrderState::Filled, 10, None, at(1))
        .unwrap();
    let row = store.order("c1").unwrap().unwrap();
    assert_eq!(row.state, OrderState::Filled);
    assert_eq!(row.filled_qty, 10);
}

// R-422
#[test]
fn lower_filled_qty_is_invalid_and_keeps_the_stored_one() {
    let store = stored();
    store
        .set_order_state("c1", OrderState::PartiallyFilled, 6, None, at(1))
        .unwrap();
    let err = store
        .set_order_state("c1", OrderState::PartiallyFilled, 4, None, at(2))
        .unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)), "got {err:?}");
    assert_eq!(store.order("c1").unwrap().unwrap().filled_qty, 6);
}

// R-422
#[test]
fn repeating_the_same_filled_qty_is_accepted() {
    let store = stored();
    store
        .set_order_state("c1", OrderState::PartiallyFilled, 6, None, at(1))
        .unwrap();
    store
        .set_order_state("c1", OrderState::PartiallyFilled, 6, None, at(2))
        .unwrap();
    assert_eq!(store.order("c1").unwrap().unwrap().filled_qty, 6);
}
