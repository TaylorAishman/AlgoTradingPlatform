//! R-476, R-477: an opening leg is abandoned when its closing leg ends short
//! after a partial fill, with warnings and no halt.

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

/// The R-292 setup (shared by R-293): sell 10 SPY (leg 1) and sell 5 SPY (leg 2) for one decision,
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

fn alerts_with(fx: &LiveFixture, code: &str) -> Vec<(String, String, String, bool)> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|(_, c, _, _)| c == code)
        .collect()
}

/// Leg 1 fills 4 of 10, then ends the way `end` says; one settle follows.
fn short_leg_one(end: fn(&LiveFixture)) -> LiveFixture {
    let fx = legs_fixture();
    submit_planned(&fx.ctx(), &auth()).expect("first submit");
    fx.mock.fill(LEG1_ID, 4, "100");
    end(&fx);
    settle(&fx.ctx()).expect("settle");
    fx
}

fn expire(fx: &LiveFixture) {
    fx.mock.expire(LEG1_ID);
}

fn cancel(fx: &LiveFixture) {
    fx.mock.set_order_status(LEG1_ID, "canceled");
}

fn check_abandoned(fx: &LiveFixture, leg1_state: OrderState) {
    let leg1 = fx.store.order(LEG1_ID).expect("read").expect("leg 1 row");
    assert_eq!(leg1.state, leg1_state);
    assert_eq!(leg1.filled_qty, 4);
    let leg2 = fx.store.order(LEG2_ID).expect("read").expect("leg 2 row");
    assert_eq!(leg2.state, OrderState::Abandoned);
    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.get(&sym("SPY")), Some(&6));

    submit_planned(&fx.ctx(), &auth()).expect("second submit");
    assert_eq!(posted_ids(fx), vec![LEG1_ID.to_string()]);
}

// R-476
#[test]
fn the_opening_leg_is_abandoned_when_the_closing_leg_expires_after_a_partial_fill() {
    let fx = short_leg_one(expire);
    check_abandoned(&fx, OrderState::Expired);
}

// R-476
#[test]
fn the_opening_leg_is_abandoned_when_the_closing_leg_is_cancelled_after_a_partial_fill() {
    let fx = short_leg_one(cancel);
    check_abandoned(&fx, OrderState::Canceled);
}

// R-477
#[test]
fn a_short_closing_leg_raises_one_incomplete_fill_and_one_order_abandoned_warning() {
    for end in [expire as fn(&LiveFixture), cancel] {
        let fx = short_leg_one(end);
        let all = fx.store.alerts().expect("alerts");
        assert_eq!(all.len(), 2, "alerts: {all:?}");
        for code in ["incomplete_fill", "order_abandoned"] {
            let found = alerts_with(&fx, code);
            assert_eq!(found.len(), 1, "{code}: {all:?}");
            assert_eq!(found[0].0, "warn");
        }
    }
}

// R-477
#[test]
fn a_short_closing_leg_sets_no_halt() {
    let fx = short_leg_one(expire);
    assert!(fx.store.active_halts().expect("halts").is_empty());
    settle(&fx.ctx()).expect("settle again");
    assert!(fx.store.active_halts().expect("halts").is_empty());
    assert_eq!(fx.store.alerts().expect("alerts").len(), 2);
}
