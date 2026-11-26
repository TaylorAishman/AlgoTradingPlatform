//! R-321: the owner can cancel every open order the system created.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::error::GateError;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::error::LiveError;
use pt_live::killswitch::cancel_open;
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

fn deletes(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "DELETE")
        .count()
}

fn assert_both_canceled_after_settle(fx: &LiveFixture) {
    settle(&fx.ctx()).expect("settle");
    for id in [TLT_ID, SPY_ID] {
        let row = fx.store.order(id).expect("read").expect("row");
        assert_eq!(row.state, OrderState::Canceled, "{id}");
    }
}

// R-321
#[test]
fn cancel_open_cancels_both_open_orders() {
    let fx = submitted_fixture();

    let n = cancel_open(&fx.ctx()).expect("cancel_open");

    assert_eq!(n, 2);
    assert_eq!(deletes(&fx), 2, "two DELETE requests");
    assert_both_canceled_after_settle(&fx);
}

// R-321
#[test]
fn cancel_open_works_while_halted() {
    let fx = submitted_fixture();
    fx.store
        .set_halt("manual", "halted for the test", fx.now())
        .expect("halt");
    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);

    let n = cancel_open(&fx.ctx()).expect("cancel_open while halted");

    assert_eq!(n, 2);
    assert_eq!(deletes(&fx), 2, "two DELETE requests");
    assert_both_canceled_after_settle(&fx);
    assert_eq!(
        fx.store.active_halts().expect("halts").len(),
        1,
        "the halt is untouched"
    );
}

// R-321
#[test]
fn in_dry_run_cancel_open_is_forbidden_and_sends_nothing() {
    let fx = LiveFixture::new(Stage::DryRun);

    let result = cancel_open(&fx.ctx());

    assert!(
        matches!(result, Err(LiveError::Gate(GateError::StageForbids))),
        "got {result:?}"
    );
    assert_eq!(deletes(&fx), 0, "no DELETE");
}

// R-321
#[test]
fn an_order_already_filled_at_the_broker_is_skipped_and_the_rest_are_cancelled() {
    let fx = submitted_fixture();
    // TLT is stored first and still reads Submitted, but has filled at the broker.
    fx.mock.fill(TLT_ID, 10, "100");

    let n = cancel_open(&fx.ctx()).expect("cancel_open");

    assert_eq!(n, 1, "only the open SPY order counts");
    assert_eq!(deletes(&fx), 2, "both cancels were tried");
    settle(&fx.ctx()).expect("settle");
    let spy = fx.store.order(SPY_ID).expect("read").expect("row");
    assert_eq!(spy.state, OrderState::Canceled);
    let tlt = fx.store.order(TLT_ID).expect("read").expect("row");
    assert_eq!(tlt.state, OrderState::Filled);
}
