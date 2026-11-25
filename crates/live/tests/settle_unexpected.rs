//! R-294: a broker order status the design never causes halts the system.

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

// R-294
#[test]
fn a_replaced_order_sets_an_unexpected_order_status_halt_naming_the_order() {
    let fx = submitted_fixture();
    fx.mock.set_order_status(SPY_ID, "replaced");

    settle(&fx.ctx()).expect("settle returns Ok");

    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1, "halts: {halts:?}");
    assert_eq!(halts[0].reason, "unexpected_order_status");
    assert!(
        halts[0].detail.contains(SPY_ID),
        "detail was {:?}",
        halts[0].detail
    );
}

// R-294
#[test]
fn a_replaced_order_keeps_its_submitted_state() {
    let fx = submitted_fixture();
    fx.mock.set_order_status(SPY_ID, "replaced");

    settle(&fx.ctx()).expect("settle");

    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Submitted);
}

// R-294
#[test]
fn a_pending_replace_order_also_halts() {
    let fx = submitted_fixture();
    fx.mock.set_order_status(TLT_ID, "pending_replace");

    settle(&fx.ctx()).expect("settle returns Ok");

    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1, "halts: {halts:?}");
    assert_eq!(halts[0].reason, "unexpected_order_status");
    assert!(halts[0].detail.contains(TLT_ID));
}

// R-294
#[test]
fn an_ordinary_status_sets_no_halt() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 10, "141");

    settle(&fx.ctx()).expect("settle");

    assert!(fx.store.active_halts().expect("halts").is_empty());
}
