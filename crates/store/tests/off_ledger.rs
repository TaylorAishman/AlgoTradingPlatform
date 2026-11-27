//! R-424: an order can be marked as outside the paper ledger.

use std::str::FromStr;

use chrono::NaiveDate;
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

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

// R-424
#[test]
fn a_new_order_is_not_off_ledger() {
    let store = stored();
    assert!(!store.is_off_ledger("c1").unwrap());
}

// R-424
#[test]
fn marking_sets_the_flag_and_leaves_the_order_row_unchanged() {
    let store = stored();
    let before = store.order("c1").unwrap();
    store.mark_off_ledger("c1").unwrap();
    assert!(store.is_off_ledger("c1").unwrap());
    assert_eq!(store.order("c1").unwrap(), before);
}

// R-424
#[test]
fn marking_twice_is_ok() {
    let store = stored();
    store.mark_off_ledger("c1").unwrap();
    store.mark_off_ledger("c1").unwrap();
    assert!(store.is_off_ledger("c1").unwrap());
}

// R-424
#[test]
fn unknown_id_is_not_found_for_both_methods() {
    let store = stored();
    assert!(matches!(
        store.mark_off_ledger("nope").unwrap_err(),
        StoreError::NotFound(_)
    ));
    assert!(matches!(
        store.is_off_ledger("nope").unwrap_err(),
        StoreError::NotFound(_)
    ));
}
