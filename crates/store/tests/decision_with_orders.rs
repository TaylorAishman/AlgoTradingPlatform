//! R-426: a decision and its orders are written together or not at all.

use std::str::FromStr;

use chrono::{NaiveDate, TimeZone, Utc};
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::trading::{DecisionRow, Mode, OrderRow};
use rust_decimal::Decimal;

fn decision() -> DecisionRow {
    DecisionRow {
        id: 0,
        strategy_id: "tsmom-v1".to_string(),
        signal_date: NaiveDate::from_ymd_opt(2026, 11, 30).unwrap(),
        trade_date: Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()),
        mode: Mode::Paper,
        status: "planned".to_string(),
        inputs_json: "{}".to_string(),
        outputs_json: "[]".to_string(),
        created_at: Utc.with_ymd_and_hms(2026, 11, 30, 21, 0, 0).unwrap(),
    }
}

fn order(id: &str, decision_id: Option<i64>) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id,
        symbol: Symbol::new("SPY").unwrap(),
        side: Side::Buy,
        qty: 10,
        leg: 0,
        state: OrderState::Intent,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from_str("500.25").unwrap(),
        trade_date: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
    }
}

// R-426
#[test]
fn orders_get_the_new_decision_id_whatever_they_were_passed() {
    let store = Store::open_in_memory().unwrap();
    let id = store
        .insert_decision_with_orders(&decision(), &[order("o1", None), order("o2", Some(9999))])
        .unwrap();
    assert_eq!(store.order("o1").unwrap().unwrap().decision_id, Some(id));
    assert_eq!(store.order("o2").unwrap().unwrap().decision_id, Some(id));
    let stored = store.decisions(Mode::Paper).unwrap();
    assert_eq!(stored, vec![DecisionRow { id, ..decision() }]);
}

// R-426
#[test]
fn a_duplicate_order_id_leaves_neither_decision_nor_orders() {
    let store = Store::open_in_memory().unwrap();
    store.insert_order(&order("o2", None)).unwrap();
    let res =
        store.insert_decision_with_orders(&decision(), &[order("o1", None), order("o2", None)]);
    assert!(matches!(res, Err(StoreError::Duplicate(_))), "got {res:?}");
    assert!(store.decisions(Mode::Paper).unwrap().is_empty());
    assert_eq!(store.order("o1").unwrap(), None);
}
