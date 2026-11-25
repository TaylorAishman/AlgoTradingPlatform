//! R-061: a client order id is unique. R-052: foreign keys are enforced on orders.

use std::str::FromStr;

use chrono::NaiveDate;
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::trading::{DecisionRow, Mode, OrderRow};
use rust_decimal::Decimal;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn order(id: &str, symbol: &str, state: OrderState) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: Symbol::new(symbol).unwrap(),
        side: Side::Buy,
        qty: 10,
        leg: 0,
        state,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from_str("123.456789012345678").unwrap(),
        trade_date: date(2026, 12, 1),
    }
}

// R-061
#[test]
fn second_insert_with_same_client_order_id_is_duplicate_and_keeps_first() {
    let store = Store::open_in_memory().unwrap();
    let first = order("c1", "SPY", OrderState::Intent);
    store.insert_order(&first).unwrap();
    let second = OrderRow {
        symbol: Symbol::new("QQQ").unwrap(),
        qty: 99,
        ..first.clone()
    };
    let err = store.insert_order(&second).unwrap_err();
    assert!(matches!(err, StoreError::Duplicate(_)), "got {err:?}");
    assert_eq!(store.order("c1").unwrap(), Some(first));
}

// R-061
#[test]
fn order_round_trips_every_field() {
    let store = Store::open_in_memory().unwrap();
    let decision_id = store
        .insert_decision(&DecisionRow {
            id: 0,
            strategy_id: "tsmom-v1".to_string(),
            signal_date: date(2026, 11, 30),
            trade_date: Some(date(2026, 12, 1)),
            mode: Mode::Paper,
            status: "planned".to_string(),
            inputs_json: "{}".to_string(),
            outputs_json: "{}".to_string(),
            created_at: chrono::DateTime::from_timestamp(1_796_000_000, 0).unwrap(),
        })
        .unwrap();
    let o = OrderRow {
        decision_id: Some(decision_id),
        side: Side::Sell,
        leg: 2,
        broker_order_id: Some("b-77".to_string()),
        filled_qty: 4,
        state: OrderState::PartiallyFilled,
        ..order("c2", "BRK.B", OrderState::Intent)
    };
    store.insert_order(&o).unwrap();
    assert_eq!(store.order("c2").unwrap(), Some(o));
}

// R-061
#[test]
fn order_is_none_for_unknown_id() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.order("nope").unwrap(), None);
}

// R-061
#[test]
fn orders_in_states_returns_only_matching_states() {
    let store = Store::open_in_memory().unwrap();
    store
        .insert_order(&order("a", "SPY", OrderState::Intent))
        .unwrap();
    store
        .insert_order(&order("b", "SPY", OrderState::Submitted))
        .unwrap();
    store
        .insert_order(&order("c", "SPY", OrderState::Filled))
        .unwrap();
    store
        .insert_order(&order("d", "SPY", OrderState::PartiallyFilled))
        .unwrap();
    let got = store
        .orders_in_states(&[OrderState::Submitted, OrderState::PartiallyFilled])
        .unwrap();
    let mut ids: Vec<&str> = got.iter().map(|o| o.client_order_id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["b", "d"]);
    assert!(store.orders_in_states(&[]).unwrap().is_empty());
    assert!(store
        .orders_in_states(&[OrderState::Rejected])
        .unwrap()
        .is_empty());
}

// R-052
#[test]
fn insert_order_with_missing_decision_is_a_sql_error() {
    let store = Store::open_in_memory().unwrap();
    let o = OrderRow {
        decision_id: Some(12345),
        ..order("c3", "SPY", OrderState::Intent)
    };
    let err = store.insert_order(&o).unwrap_err();
    assert!(matches!(err, StoreError::Sql(_)), "got {err:?}");
    assert_eq!(store.order("c3").unwrap(), None);
}
