//! R-503, R-504: a failed run sends nothing and raises one critical alert.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_live::submit::submit_planned;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

const ALL_STATES: [OrderState; 9] = [
    OrderState::Intent,
    OrderState::DryRun,
    OrderState::Submitted,
    OrderState::PartiallyFilled,
    OrderState::Filled,
    OrderState::Canceled,
    OrderState::Expired,
    OrderState::Rejected,
    OrderState::Abandoned,
];

fn failing_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.fail_next(100, 500);
    fx
}

fn requests_with(fx: &LiveFixture, method: &str) -> Vec<String> {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == method)
        .map(|r| format!("{r:?}"))
        .collect()
}

/// A paper fixture holding one submitted order whose broker status is a word
/// outside the broker's vocabulary.
fn unknown_status_fixture() -> LiveFixture {
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
    let auth = AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 3, 1),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    };
    submit_planned(&fx.ctx(), &auth).expect("submit");
    fx.mock.set_order_status(SPY_ID, "no_such_status_word");
    fx
}

// R-503
#[test]
fn a_failed_paper_run_has_status_failed_and_a_failed_run_row() {
    let fx = failing_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Failed);
    let rows = fx.store.runs().expect("runs");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "failed");
}

// R-503
#[test]
fn a_failed_paper_run_sends_no_post_and_no_delete() {
    let fx = failing_fixture();

    run_cycle(&fx.ctx()).expect("run");

    assert!(requests_with(&fx, "POST").is_empty());
    assert!(requests_with(&fx, "DELETE").is_empty());
}

// R-503
#[test]
fn a_failed_paper_run_leaves_no_order_row() {
    let fx = failing_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let orders = fx.store.orders_in_states(&ALL_STATES).expect("orders");
    assert!(orders.is_empty(), "{orders:?}");
}

// R-504
#[test]
fn a_broker_failure_raises_exactly_one_critical_alert_with_a_broker_or_feed_code() {
    let fx = failing_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let alerts = fx.store.alerts().expect("alerts");
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert!(
        alerts[0].1 == "broker_error" || alerts[0].1 == "feed_error",
        "{alerts:?}"
    );
    assert_eq!(alerts[0].0, "critical", "{alerts:?}");
}

// R-504
#[test]
fn an_unknown_order_status_fails_the_run_with_one_critical_run_failed_alert() {
    let fx = unknown_status_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Failed);
    let alerts = fx.store.alerts().expect("alerts");
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert_eq!(alerts[0].1, "run_failed", "{alerts:?}");
    assert_eq!(alerts[0].0, "critical", "{alerts:?}");
}

// R-504
#[test]
fn an_unknown_order_status_run_sends_no_further_post() {
    let fx = unknown_status_fixture();
    let before = requests_with(&fx, "POST").len();

    run_cycle(&fx.ctx()).expect("run");

    assert_eq!(requests_with(&fx, "POST").len(), before);
}
