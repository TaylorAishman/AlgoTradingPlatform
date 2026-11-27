//! R-478: the released-legs count covers only what this settlement released.

mod common;

use chrono::{DateTime, NaiveDate};
use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::settle::settle;
use pt_live::submit::submit_planned;
use pt_store::trading::{Book, DecisionRow, LedgerEntry, Mode, OrderRow};
use rust_decimal::Decimal;

const LEG1_ID: &str = "pt-tsmom-v1-20240229-SPY-1";
const LEG2_ID: &str = "pt-tsmom-v1-20240229-SPY-2";

fn leg(id: &str, decision_id: i64, qty: u64, leg: u8) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: Some(decision_id),
        symbol: sym("SPY"),
        side: Side::Sell,
        qty,
        leg,
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

/// POST requests to the order endpoint, as the client order id each carries.
fn posted_ids(fx: &LiveFixture) -> Vec<String> {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .map(|r| {
            let body: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
            body["client_order_id"]
                .as_str()
                .expect("client order id")
                .to_string()
        })
        .collect()
}

/// The two-leg setup: sell 10 SPY (leg 1) and sell 5 SPY (leg 2) for one decision,
/// with a broker position of +10 SPY matched by the ledger.
fn legs_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("SPY", 10);
    fx.store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: NaiveDate::from_ymd_opt(2024, 2, 29).expect("date"),
            kind: "fill".to_string(),
            symbol: Some(sym("SPY")),
            qty: 10,
            price: Some(Decimal::from(100)),
            cash_delta: Decimal::from(-1000),
            ref_id: None,
        })
        .expect("ledger");
    let decision_id = fx
        .store
        .insert_decision(&DecisionRow {
            id: 0,
            strategy_id: "tsmom-v1".to_string(),
            signal_date: d(2024, 2, 29),
            trade_date: Some(d(2024, 3, 1)),
            mode: Mode::Paper,
            status: "planned".to_string(),
            inputs_json: "{}".to_string(),
            outputs_json: "{}".to_string(),
            created_at: DateTime::from_timestamp(1_709_200_000, 0).expect("time"),
        })
        .expect("decision");
    fx.store
        .insert_order(&leg(LEG1_ID, decision_id, 10, 1))
        .expect("insert leg 1");
    fx.store
        .insert_order(&leg(LEG2_ID, decision_id, 5, 2))
        .expect("insert leg 2");
    fx
}

// R-478
#[test]
fn a_second_settle_straight_after_releases_no_legs() {
    let fx = legs_fixture();
    submit_planned(&fx.ctx(), &auth()).expect("first submit");
    fx.mock.fill(LEG1_ID, 10, "141");

    let first = settle(&fx.ctx()).expect("settle");
    let second = settle(&fx.ctx()).expect("settle again");

    assert_eq!(first.legs_released, 1);
    assert_eq!(second.legs_released, 0);
    assert_eq!(posted_ids(&fx), vec![LEG1_ID.to_string()]);
}
