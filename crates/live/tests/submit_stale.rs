//! R-285: an intent whose trading session has passed is abandoned.

mod common;

use chrono::NaiveDate;
use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

fn intent(id: &str, trade_date: NaiveDate) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: sym("SPY"),
        side: Side::Buy,
        qty: 10,
        leg: 1,
        state: OrderState::Intent,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from(100),
        trade_date,
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

const STALE_ID: &str = "pt-tsmom-v1-20240226-SPY-1";
const PREV_ID: &str = "pt-tsmom-v1-20240228-SPY-1";

// R-285
#[test]
fn an_intent_for_a_past_session_is_abandoned_and_never_sent() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(STALE_ID, d(2024, 2, 27)))
        .expect("insert");

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(report.abandoned, 1);
    assert_eq!(report.submitted, 0);
    assert_eq!(report.adopted, 0);
    assert_eq!(report.rejected, 0);
    let row = fx.store.order(STALE_ID).expect("read").expect("row exists");
    assert_eq!(row.state, OrderState::Abandoned);
    assert_eq!(row.broker_order_id, None);
    assert_eq!(posts(&fx), 0, "no POST");
}

// R-285
#[test]
fn abandoning_an_intent_records_an_order_abandoned_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(STALE_ID, d(2024, 2, 27)))
        .expect("insert");

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    let alerts = fx.store.alerts().expect("alerts");
    let abandoned: Vec<_> = alerts
        .iter()
        .filter(|(_, code, _, _)| code == "order_abandoned")
        .collect();
    assert_eq!(abandoned.len(), 1, "alerts: {alerts:?}");
}

// R-285
#[test]
fn an_intent_one_session_earlier_is_still_submitted() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(PREV_ID, d(2024, 2, 29)))
        .expect("insert");

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(report.submitted, 1);
    assert_eq!(report.abandoned, 0);
    let row = fx.store.order(PREV_ID).expect("read").expect("row exists");
    assert_eq!(row.state, OrderState::Submitted);
    assert!(row.broker_order_id.is_some());
    assert_eq!(posts(&fx), 1);
    let alerts = fx.store.alerts().expect("alerts");
    assert!(alerts
        .iter()
        .all(|(_, code, _, _)| code != "order_abandoned"));
}

// R-285
#[test]
fn a_stale_and_a_current_intent_are_handled_separately() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(STALE_ID, d(2024, 2, 27)))
        .expect("insert stale");
    fx.store
        .insert_order(&intent(PREV_ID, d(2024, 2, 29)))
        .expect("insert current");

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(report.abandoned, 1);
    assert_eq!(report.submitted, 1);
    assert_eq!(
        fx.store.order(STALE_ID).unwrap().unwrap().state,
        OrderState::Abandoned
    );
    assert_eq!(
        fx.store.order(PREV_ID).unwrap().unwrap().state,
        OrderState::Submitted
    );
    assert_eq!(posts(&fx), 1);
}
