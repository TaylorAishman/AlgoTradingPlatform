//! R-480: an order that ended short is announced once, however often settle runs.

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

const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

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

fn incomplete_alerts(fx: &LiveFixture) -> usize {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|(_, code, _, _)| code == "incomplete_fill")
        .count()
}

// R-480
#[test]
fn a_short_ended_order_has_exactly_one_incomplete_fill_alert_after_every_settle() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&OrderRow {
            client_order_id: SPY_ID.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Buy,
            qty: 10,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::from(100),
            trade_date: d(2024, 3, 1),
        })
        .expect("insert");
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    fx.mock.fill(SPY_ID, 4, "141");
    fx.mock.expire(SPY_ID);

    settle(&fx.ctx()).expect("first settle");
    assert_eq!(incomplete_alerts(&fx), 1);
    settle(&fx.ctx()).expect("second settle");
    assert_eq!(incomplete_alerts(&fx), 1);
    settle(&fx.ctx()).expect("third settle");
    assert_eq!(incomplete_alerts(&fx), 1);
}
