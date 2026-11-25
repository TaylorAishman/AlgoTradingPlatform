//! R-291: an order that ends short of its quantity raises an alert and is not resent.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::settle::settle;
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

const TLT_ID: &str = "pt-tsmom-v1-20240229-TLT-1";
const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

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

/// The R-283 setup: a TLT sell and an SPY buy, both submitted.
fn submitted_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    fx
}

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST")
        .count()
}

fn incomplete_alerts(fx: &LiveFixture) -> Vec<(String, String, String, bool)> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|(_, code, _, _)| code == "incomplete_fill")
        .collect()
}

// R-291
#[test]
fn an_expired_short_fill_is_expired_with_its_filled_quantity_and_alerts_naming_the_symbol() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");
    fx.mock.expire(SPY_ID);

    settle(&fx.ctx()).expect("settle");

    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Expired);
    assert_eq!(spy.filled_qty, 4);
    let alerts = incomplete_alerts(&fx);
    assert_eq!(alerts.len(), 1, "alerts: {alerts:?}");
    assert!(alerts[0].2.contains("SPY"), "message: {}", alerts[0].2);
}

// R-291
#[test]
fn a_cancelled_short_fill_also_alerts() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");
    fx.mock.set_order_status(SPY_ID, "canceled");

    settle(&fx.ctx()).expect("settle");

    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Canceled);
    assert_eq!(spy.filled_qty, 4);
    let alerts = incomplete_alerts(&fx);
    assert_eq!(alerts.len(), 1, "alerts: {alerts:?}");
    assert!(alerts[0].2.contains("SPY"), "message: {}", alerts[0].2);
}

// R-291
#[test]
fn repeated_settles_raise_the_alert_exactly_once() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");
    fx.mock.expire(SPY_ID);

    settle(&fx.ctx()).expect("first settle");
    settle(&fx.ctx()).expect("second settle");
    settle(&fx.ctx()).expect("third settle");

    let alerts = incomplete_alerts(&fx);
    assert_eq!(alerts.len(), 1, "alerts: {alerts:?}");
}

// R-291
#[test]
fn a_short_ended_order_is_never_resent() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");
    fx.mock.expire(SPY_ID);
    settle(&fx.ctx()).expect("settle");
    let posts_before = posts(&fx);
    assert_eq!(posts_before, 2, "TLT and SPY were posted once each");

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit again");

    assert_eq!(report.submitted, 0);
    assert_eq!(posts(&fx), posts_before, "no new POST");
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Expired);
    assert_eq!(spy.filled_qty, 4);
}

// R-291
#[test]
fn a_fully_filled_order_raises_no_incomplete_fill_alert() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 10, "141");

    settle(&fx.ctx()).expect("settle");

    assert!(incomplete_alerts(&fx).is_empty());
}

// R-291
#[test]
fn an_order_still_open_after_a_partial_fill_raises_no_alert_yet() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");

    settle(&fx.ctx()).expect("settle");

    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::PartiallyFilled);
    assert!(incomplete_alerts(&fx).is_empty());
}
