//! R-283: an order is written before it is sent and marked submitted after the broker accepts it.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
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

/// The requests that touch orders, as (method, client order id or empty).
fn order_requests(fx: &LiveFixture) -> Vec<(String, String)> {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.path.starts_with("/v2/orders"))
        .map(|r| {
            let id = if r.method == "GET" {
                r.query
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .find(|(k, _)| *k == "client_order_id")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default()
            } else {
                serde_json::from_str::<serde_json::Value>(&r.body)
                    .ok()
                    .and_then(|v| v["client_order_id"].as_str().map(str::to_string))
                    .unwrap_or_default()
            };
            (r.method, id)
        })
        .collect()
}

// R-283
#[test]
fn two_intents_are_submitted_and_marked_submitted_with_broker_ids() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");

    let report = submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(report.submitted, 2);
    assert_eq!(report.adopted, 0);
    assert_eq!(report.rejected, 0);
    assert_eq!(report.abandoned, 0);
    for id in [TLT_ID, SPY_ID] {
        let row = fx.store.order(id).expect("read").expect("row exists");
        assert_eq!(row.state, OrderState::Submitted, "{id}");
        assert!(
            row.broker_order_id
                .as_deref()
                .is_some_and(|b| !b.is_empty()),
            "{id} has a broker order id"
        );
    }
}

// R-283
#[test]
fn each_order_is_looked_up_by_client_id_then_posted_in_plan_order() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    let get = |id: &str| ("GET".to_string(), id.to_string());
    let post = |id: &str| ("POST".to_string(), id.to_string());
    assert_eq!(
        order_requests(&fx),
        vec![get(TLT_ID), post(TLT_ID), get(SPY_ID), post(SPY_ID)]
    );
}

// R-283
#[test]
fn posted_orders_carry_the_stored_symbol_side_and_quantity() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    let bodies: Vec<serde_json::Value> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .map(|r| serde_json::from_str(&r.body).expect("json body"))
        .collect();
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0]["symbol"], "TLT");
    assert_eq!(bodies[0]["side"], "sell");
    assert_eq!(bodies[1]["symbol"], "SPY");
    assert_eq!(bodies[1]["side"], "buy");
    for b in &bodies {
        assert_eq!(b["qty"].to_string().trim_matches('"'), "10");
    }
}
