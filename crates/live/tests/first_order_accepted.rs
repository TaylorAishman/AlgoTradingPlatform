//! R-493: a first order is used up only once the broker accepted one.

mod common;

use chrono::{TimeZone, Utc};
use common::{d, sym};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_live::first_order::first_order_accepted;
use pt_store::db::Store;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

fn put(store: &Store, id: &str, path: &[(OrderState, u64)]) {
    store
        .insert_order(&OrderRow {
            client_order_id: id.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Buy,
            qty: 2,
            leg: 0,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::ZERO,
            trade_date: d(2024, 3, 1),
        })
        .expect("insert");
    let at = Utc.with_ymd_and_hms(2024, 3, 1, 15, 0, 0).unwrap();
    for (state, filled) in path {
        store
            .set_order_state(id, *state, *filled, Some("broker-1"), at)
            .expect("transition");
    }
}

// R-493
#[test]
fn an_empty_store_has_no_accepted_first_order() {
    let store = Store::open_in_memory().expect("store");
    assert!(!first_order_accepted(&store).expect("accepted"));
}

// R-493
#[test]
fn an_intent_first_order_is_not_accepted() {
    let store = Store::open_in_memory().expect("store");
    put(&store, "pt-first-20240301-SPY", &[]);
    assert!(!first_order_accepted(&store).expect("accepted"));
}

// R-493
#[test]
fn a_rejected_first_order_is_not_accepted() {
    let store = Store::open_in_memory().expect("store");
    put(
        &store,
        "pt-first-20240301-SPY",
        &[(OrderState::Rejected, 0)],
    );
    put(
        &store,
        "pt-first-20240301-SPY-r1",
        &[(OrderState::Rejected, 0)],
    );
    assert!(!first_order_accepted(&store).expect("accepted"));
}

// R-493
#[test]
fn a_first_order_in_an_accepted_state_is_accepted() {
    let cases: [(&str, Vec<(OrderState, u64)>); 5] = [
        ("submitted", vec![(OrderState::Submitted, 0)]),
        (
            "partially filled",
            vec![(OrderState::Submitted, 0), (OrderState::PartiallyFilled, 1)],
        ),
        (
            "filled",
            vec![(OrderState::Submitted, 0), (OrderState::Filled, 2)],
        ),
        (
            "canceled",
            vec![(OrderState::Submitted, 0), (OrderState::Canceled, 0)],
        ),
        (
            "expired",
            vec![(OrderState::Submitted, 0), (OrderState::Expired, 0)],
        ),
    ];
    for (name, path) in cases {
        let store = Store::open_in_memory().expect("store");
        put(&store, "pt-first-20240301-SPY", &path);
        assert!(first_order_accepted(&store).expect("accepted"), "{name}");
    }
}

// R-493
#[test]
fn a_retry_row_accepted_after_rejections_counts() {
    let store = Store::open_in_memory().expect("store");
    put(
        &store,
        "pt-first-20240301-SPY",
        &[(OrderState::Rejected, 0)],
    );
    put(
        &store,
        "pt-first-20240301-SPY-r1",
        &[(OrderState::Submitted, 0)],
    );
    assert!(first_order_accepted(&store).expect("accepted"));
}

// R-493
#[test]
fn other_strategies_orders_do_not_make_it_accepted() {
    let store = Store::open_in_memory().expect("store");
    put(
        &store,
        "pt-tsmom-20240301-SPY",
        &[(OrderState::Submitted, 0), (OrderState::Filled, 2)],
    );
    put(
        &store,
        "pt-flatten-20240301-SPY",
        &[(OrderState::Submitted, 0), (OrderState::Filled, 2)],
    );
    assert!(!first_order_accepted(&store).expect("accepted"));
}
