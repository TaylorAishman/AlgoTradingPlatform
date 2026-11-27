//! R-484: submission fails when the staleness of an intent cannot be judged.

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

const ID: &str = "pt-tsmom-v1-20240102-SPY-1";

fn auth() -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 1, 2),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    }
}

// R-484
#[test]
fn a_trade_session_with_no_session_before_it_fails_submission_and_sends_nothing() {
    let fx = LiveFixture::new(Stage::Paper);
    // The fixture calendar starts at 2024-01-02; make that the trade session.
    fx.mock.set_clock(
        "2024-01-02T08:00:00-05:00",
        false,
        "2024-01-02T09:30:00-05:00",
        "2024-01-02T16:00:00-05:00",
    );
    fx.store
        .insert_order(&OrderRow {
            client_order_id: ID.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Buy,
            qty: 10,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::from(100),
            trade_date: d(2024, 1, 2),
        })
        .expect("insert");

    let err = submit_planned(&fx.ctx(), &auth()).expect_err("submission must fail");

    assert!(matches!(err, LiveError::Invalid(_)), "{err:?}");
    let row = fx.store.order(ID).expect("read").expect("row exists");
    assert_eq!(row.state, OrderState::Intent);
    let posts = fx
        .mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST")
        .count();
    assert_eq!(posts, 0, "no POST");
}
