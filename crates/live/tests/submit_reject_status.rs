//! R-482, R-483: only a 403 or 422 on the order POST is an order reject; any other error fails the submission.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::error::LiveError;
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

const TLT_ID: &str = "pt-tsmom-v1-20240301-TLT-1";
const SPY_ID: &str = "pt-tsmom-v1-20240301-SPY-1";

fn intent(id: &str, symbol: &str, side: Side) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: sym(symbol),
        side,
        qty: 10,
        leg: 1,
        state: OrderState::Intent,
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

fn setup() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");
    fx
}

fn order_posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .count()
}

fn rejected_alerts(fx: &LiveFixture) -> usize {
    fx.store
        .alerts()
        .expect("alerts")
        .iter()
        .filter(|(_, code, _, _)| code == "order_rejected")
        .count()
}

fn assert_rejects_one_and_goes_on(status: u16) {
    let fx = setup();
    fx.mock.reject_next("POST", "/v2/orders", 1, status);

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit carries on");

    assert_eq!(report.rejected, 1, "status {status}");
    assert_eq!(report.submitted, 1, "status {status}");
    let tlt = fx.store.order(TLT_ID).expect("read").expect("TLT row");
    assert_eq!(tlt.state, OrderState::Rejected);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Submitted);
    assert_eq!(rejected_alerts(&fx), 1);
}

fn assert_fails_and_leaves_intent(status: u16) {
    let fx = setup();
    fx.mock.reject_next("POST", "/v2/orders", 1, status);

    let err = submit_planned(&fx.ctx(), &auth()).expect_err("submission must fail");

    assert!(
        matches!(err, LiveError::Alpaca(_)),
        "status {status}: {err:?}"
    );
    let tlt = fx.store.order(TLT_ID).expect("read").expect("TLT row");
    assert_eq!(tlt.state, OrderState::Intent, "status {status}");
    assert_eq!(rejected_alerts(&fx), 0, "status {status}");
    assert_eq!(order_posts(&fx), 1, "status {status}");
}

// R-482
#[test]
fn a_422_on_the_order_post_rejects_that_order_and_the_other_goes_on() {
    assert_rejects_one_and_goes_on(422);
}

// R-482
#[test]
fn a_403_on_the_order_post_rejects_that_order_and_the_other_goes_on() {
    assert_rejects_one_and_goes_on(403);
}

// R-483
#[test]
fn a_401_on_the_order_post_fails_the_submission_and_leaves_the_intent() {
    assert_fails_and_leaves_intent(401);
}

// R-483
#[test]
fn a_400_on_the_order_post_fails_the_submission_and_leaves_the_intent() {
    assert_fails_and_leaves_intent(400);
}
