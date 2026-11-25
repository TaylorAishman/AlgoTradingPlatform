//! R-288: settlement brings every open order's state up to date.

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

// R-288
#[test]
fn a_filled_order_is_updated_and_an_untouched_one_is_left_submitted() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 10, "141");

    let report = settle(&fx.ctx()).expect("settle");

    assert_eq!(report.orders_updated, 1);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Filled);
    assert_eq!(spy.filled_qty, 10);
    let tlt = fx.store.order(TLT_ID).expect("read").expect("TLT row");
    assert_eq!(tlt.state, OrderState::Submitted);
}

// R-288
#[test]
fn a_second_settle_updates_nothing() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 10, "141");
    settle(&fx.ctx()).expect("first settle");

    let report = settle(&fx.ctx()).expect("second settle");

    assert_eq!(report.orders_updated, 0);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Filled);
    assert_eq!(spy.filled_qty, 10);
}
