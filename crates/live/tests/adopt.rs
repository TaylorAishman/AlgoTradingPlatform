//! R-472, R-473: a rejected order the broker still holds is adopted; any other is left alone.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::adopt::adopt_if_held;
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

const ID: &str = "pt-tsmom-v1-20240301-SPY-1";

fn row(state: OrderState) -> OrderRow {
    OrderRow {
        client_order_id: ID.to_string(),
        decision_id: None,
        symbol: sym("SPY"),
        side: Side::Buy,
        qty: 10,
        leg: 1,
        state,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from(100),
        trade_date: d(2024, 3, 1),
    }
}

fn auth() -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 3, 1),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    }
}

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.starts_with("/v2/orders"))
        .count()
}

fn lookups(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "GET" && r.path.contains("by_client_order_id"))
        .count()
}

/// A stored `Rejected` row whose order the mock holds, as after a failed POST response.
fn rejected_but_held() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&row(OrderState::Intent))
        .expect("insert");
    fx.mock.reject_but_keep_next_order(422);
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    let stored = fx.store.order(ID).expect("read").expect("row");
    assert_eq!(stored.state, OrderState::Rejected);
    fx
}

// R-472
#[test]
fn a_rejected_order_the_broker_holds_is_adopted_without_a_new_post() {
    let fx = rejected_but_held();
    let posts_before = posts(&fx);
    let lookups_before = lookups(&fx);
    let total_before = fx.mock.requests().len();

    let adopted = adopt_if_held(&fx.ctx(), ID).expect("adopt");

    assert!(adopted);
    let stored = fx.store.order(ID).expect("read").expect("row");
    assert_eq!(stored.state, OrderState::Submitted);
    let broker_id = stored.broker_order_id.expect("broker order id stored");
    assert!(!broker_id.is_empty());
    assert_eq!(posts(&fx), posts_before, "no POST");
    assert_eq!(lookups(&fx) - lookups_before, 1, "one lookup by client id");
    assert_eq!(
        fx.mock.requests().len() - total_before,
        1,
        "nothing else asked"
    );
}

// R-473
#[test]
fn an_order_the_broker_does_not_know_is_not_adopted() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&row(OrderState::Rejected))
        .expect("insert");

    let adopted = adopt_if_held(&fx.ctx(), ID).expect("adopt");

    assert!(!adopted);
    let stored = fx.store.order(ID).expect("read").expect("row");
    assert_eq!(stored.state, OrderState::Rejected);
    assert_eq!(stored.broker_order_id, None);
    assert_eq!(posts(&fx), 0);
}

// R-473
#[test]
fn an_order_the_broker_holds_as_rejected_is_not_adopted() {
    let fx = rejected_but_held();
    fx.mock.set_order_status(ID, "rejected");

    let adopted = adopt_if_held(&fx.ctx(), ID).expect("adopt");

    assert!(!adopted);
    let stored = fx.store.order(ID).expect("read").expect("row");
    assert_eq!(stored.state, OrderState::Rejected);
}

// R-473
#[test]
fn a_row_that_is_not_rejected_is_not_adopted_and_no_request_is_made() {
    for state in [OrderState::Intent, OrderState::Submitted] {
        let fx = LiveFixture::new(Stage::Paper);
        fx.store.insert_order(&row(state)).expect("insert");
        fx.mock.seed_foreign_order(ID, "SPY", "buy", 10);
        let before = fx.mock.requests().len();

        let adopted = adopt_if_held(&fx.ctx(), ID).expect("adopt");

        assert!(!adopted, "{state:?}");
        let stored = fx.store.order(ID).expect("read").expect("row");
        assert_eq!(stored.state, state);
        assert_eq!(fx.mock.requests().len(), before, "{state:?}: no request");
    }
}
