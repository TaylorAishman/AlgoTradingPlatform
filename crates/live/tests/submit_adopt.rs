//! R-284: an order the broker already has is adopted, not sent again.

mod common;

use common::{d, sym, LiveFixture};
use pt_alpaca::orders_read::order_by_client_id;
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

fn intent() -> OrderRow {
    OrderRow {
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

// R-284
#[test]
fn an_order_the_broker_already_holds_is_adopted_and_not_posted_again() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store.insert_order(&intent()).expect("insert");
    fx.mock.seed_foreign_order(SPY_ID, "SPY", "buy", 10);
    let broker_id = order_by_client_id(&fx.http, SPY_ID)
        .expect("lookup")
        .expect("broker holds the order")
        .id;
    let posts_before = posts(&fx);

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(report.adopted, 1);
    assert_eq!(report.submitted, 0);
    assert_eq!(report.rejected, 0);
    assert_eq!(report.abandoned, 0);
    let row = fx.store.order(SPY_ID).expect("read").expect("row exists");
    assert_eq!(row.state, OrderState::Submitted);
    assert_eq!(row.broker_order_id.as_deref(), Some(broker_id.as_str()));
    assert_eq!(posts(&fx), posts_before, "no further POST");
    assert_eq!(posts_before, 0);
}

// R-284
#[test]
fn a_second_call_does_not_adopt_or_send_the_same_order_again() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store.insert_order(&intent()).expect("insert");
    fx.mock.seed_foreign_order(SPY_ID, "SPY", "buy", 10);

    let first = submit_planned(&fx.ctx(), &auth()).expect("first submit");
    assert_eq!(first.adopted, 1);
    let report = submit_planned(&fx.ctx(), &auth()).expect("second submit");

    assert_eq!(report.adopted, 0);
    assert_eq!(report.submitted, 0);
    assert_eq!(posts(&fx), 0);
}
