//! R-319, R-320: `first_order` plans the owner's approved order, and only on its day.

mod common;

use common::{d, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_live::first_order::first_order;
use pt_store::trading::Mode;

/// A first-order-stage fixture whose approval is valid on `valid_on`.
fn approved_fixture(valid_on: chrono::NaiveDate) -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::FirstOrder);
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            max_qty: 1,
            valid_on,
        }),
        ..OwnerFile::default()
    });
    fx
}

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST")
        .count()
}

// R-319
#[test]
fn plans_the_approved_order_as_an_intent_on_its_day() {
    let fx = approved_fixture(d(2024, 3, 1));
    let wire = clock(&fx.http).expect("clock");
    assert!(first_order(&fx.ctx(), &wire).expect("first_order"));

    let intents = fx
        .store
        .orders_in_states(&[OrderState::Intent])
        .expect("orders");
    assert_eq!(intents.len(), 1);
    let order = &intents[0];
    assert_eq!(order.client_order_id, "pt-first-20240301-SPY");
    assert_eq!(order.symbol, sym("SPY"));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.qty, 1);
    assert_eq!(order.decision_id, None);
    assert_eq!(order.trade_date, d(2024, 3, 1));
}

// R-319
#[test]
fn planning_the_first_order_writes_no_decision_and_sends_nothing() {
    let fx = approved_fixture(d(2024, 3, 1));
    let wire = clock(&fx.http).expect("clock");
    first_order(&fx.ctx(), &wire).expect("first_order");

    for mode in [Mode::Paper, Mode::DryRun] {
        assert!(fx.store.decisions(mode).expect("decisions").is_empty());
    }
    assert_eq!(posts(&fx), 0);
}

// R-320
#[test]
fn plans_nothing_when_valid_on_is_not_the_trade_session() {
    let fx = approved_fixture(d(2024, 3, 4));
    let wire = clock(&fx.http).expect("clock");
    assert!(!first_order(&fx.ctx(), &wire).expect("first_order"));

    assert!(fx
        .store
        .orders_in_states(&[OrderState::Intent, OrderState::Submitted])
        .expect("orders")
        .is_empty());
    assert!(fx
        .store
        .order("pt-first-20240304-SPY")
        .expect("lookup")
        .is_none());
    assert_eq!(posts(&fx), 0);
}

// R-319
#[test]
fn plans_nothing_without_an_approval() {
    let mut fx = approved_fixture(d(2024, 3, 1));
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        ..OwnerFile::default()
    });
    let wire = clock(&fx.http).expect("clock");
    assert!(!first_order(&fx.ctx(), &wire).expect("first_order"));
    assert!(fx
        .store
        .orders_in_states(&[OrderState::Intent])
        .expect("orders")
        .is_empty());
}
