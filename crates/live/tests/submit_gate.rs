//! R-287: submission goes through the gate.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::error::GateError;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::error::LiveError;
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

const ID: &str = "pt-tsmom-v1-20240301-SPY-1";

fn intent() -> OrderRow {
    OrderRow {
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
        trade_date: d(2024, 3, 1),
    }
}

fn auth(halted: bool) -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 3, 1),
        halted,
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

// R-287
#[test]
fn in_dry_run_the_gate_refuses_and_the_order_stays_intent() {
    let fx = LiveFixture::new(Stage::DryRun);
    fx.store.insert_order(&intent()).expect("insert");

    let result = submit_planned(&fx.ctx(), &auth(false));

    assert!(
        matches!(result, Err(LiveError::Gate(GateError::StageForbids))),
        "got {result:?}"
    );
    let row = fx.store.order(ID).expect("read").expect("row");
    assert_eq!(row.state, OrderState::Intent);
    assert_eq!(posts(&fx), 0, "no POST");
}

// R-287
#[test]
fn an_active_halt_makes_the_gate_refuse_and_nothing_is_sent() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store.insert_order(&intent()).expect("insert");
    fx.store
        .set_halt("test_halt", "halted for the test", fx.now())
        .expect("halt");
    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);

    let result = submit_planned(&fx.ctx(), &auth(true));

    assert!(
        matches!(result, Err(LiveError::Gate(GateError::Halted))),
        "got {result:?}"
    );
    let row = fx.store.order(ID).expect("read").expect("row");
    assert_eq!(row.state, OrderState::Intent);
    assert_eq!(posts(&fx), 0, "no POST");
}
