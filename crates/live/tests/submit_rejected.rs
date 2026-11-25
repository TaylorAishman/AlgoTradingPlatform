//! R-286: a broker rejection marks that order rejected and leaves the others alone.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
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
    // TLT is no longer shortable at the broker, so selling it is refused.
    fx.mock.seed_asset("TLT", true, false, false);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");
    fx
}

// R-286
#[test]
fn a_rejected_order_is_marked_rejected_and_the_next_one_is_still_sent() {
    let fx = setup();

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit carries on");

    assert_eq!(report.rejected, 1);
    assert_eq!(report.submitted, 1);
    assert_eq!(report.adopted, 0);
    assert_eq!(report.abandoned, 0);
    let tlt = fx.store.order(TLT_ID).expect("read").expect("TLT row");
    assert_eq!(tlt.state, OrderState::Rejected);
    assert_eq!(tlt.broker_order_id, None);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Submitted);
    assert!(spy.broker_order_id.is_some());
}

// R-286
#[test]
fn a_rejection_records_an_order_rejected_alert_naming_the_symbol() {
    let fx = setup();

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    let alerts = fx.store.alerts().expect("alerts");
    let rejected: Vec<_> = alerts
        .iter()
        .filter(|(_, code, _, _)| code == "order_rejected")
        .collect();
    assert_eq!(rejected.len(), 1, "alerts: {alerts:?}");
    assert!(rejected[0].2.contains("TLT"), "message: {}", rejected[0].2);
}

// R-286
#[test]
fn a_rejection_does_not_raise_alerts_for_the_accepted_order() {
    let fx = setup();

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    let alerts = fx.store.alerts().expect("alerts");
    assert!(
        alerts.iter().all(|(_, _, msg, _)| !msg.contains("SPY")),
        "alerts: {alerts:?}"
    );
}
