//! R-520: the shadow day fills the orders that reached the broker and no others.

mod common;

use common::*;
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_shadow::daily::shadow_day;
use pt_store::db::Store;

fn within(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

fn base_return(store: &Store) -> f64 {
    shadow_day(store, &config(), d(2024, 1, 3))
        .unwrap()
        .shadow_return
}

/// Extra buy of 100 A for 2024-01-03 inserted directly in `state`.
fn insert_extra(store: &Store, state: OrderState) {
    store
        .insert_order(&order("o-extra", "A", Side::Buy, 100, state, None, "100"))
        .unwrap();
}

// R-520
#[test]
fn orders_that_never_reached_the_broker_leave_the_shadow_return_unchanged() {
    for state in [
        OrderState::Intent,
        OrderState::DryRun,
        OrderState::Rejected,
        OrderState::Abandoned,
    ] {
        let store = shadow_store();
        let before = base_return(&store);
        insert_extra(&store, state);
        within(base_return(&store), before);
    }
}

// R-520
#[test]
fn canceled_and_expired_orders_are_filled_in_full_at_the_raw_open() {
    // 100 A bought at the raw open 101 (cost 10 bps of 10100 = 10.1), marked at
    // the raw close 102: +100 - 10.1 on a start equity of 100000.
    let expected_delta = (100.0 - 10.1) / 100000.0;
    for state in [OrderState::Canceled, OrderState::Expired] {
        let store = shadow_store();
        let before = base_return(&store);
        insert_extra(&store, OrderState::Intent);
        store
            .set_order_state(
                "o-extra",
                OrderState::Submitted,
                0,
                Some("b-x"),
                at(2024, 1, 3, 14, 30),
            )
            .unwrap();
        store
            .set_order_state("o-extra", state, 0, None, at(2024, 1, 3, 20, 0))
            .unwrap();
        within(base_return(&store), before + expected_delta);
    }
}
