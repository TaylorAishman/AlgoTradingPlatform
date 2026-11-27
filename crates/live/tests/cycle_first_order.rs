//! R-319: in the first-order stage the cycle sends the approved order and nothing else.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::Mode;

fn approved_fixture() -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::FirstOrder);
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty: 1,
            max_qty: 1,
            valid_on: d(2024, 3, 1),
        }),
        ..OwnerFile::default()
    });
    fx
}

// R-319
#[test]
fn a_first_order_cycle_submits_the_approved_order_and_ends_ok() {
    let fx = approved_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    let orders = fx
        .store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::DryRun,
            OrderState::Rejected,
        ])
        .expect("orders");
    assert_eq!(orders.len(), 1, "{orders:?}");
    let order = &orders[0];
    assert_eq!(order.client_order_id, "pt-first-20240301-SPY");
    assert_eq!(order.symbol, sym("SPY"));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.qty, 1);
    assert_eq!(order.state, OrderState::Submitted);
    assert_eq!(order.decision_id, None);
}

// R-319
#[test]
fn a_first_order_cycle_posts_exactly_one_order() {
    let fx = approved_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let posts = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .count();
    assert_eq!(posts, 1);
}

// R-319
#[test]
fn a_first_order_cycle_writes_no_rebalance_decision() {
    let fx = approved_fixture();

    run_cycle(&fx.ctx()).expect("run");

    for mode in [Mode::Paper, Mode::DryRun] {
        assert!(fx.store.decisions(mode).expect("decisions").is_empty());
    }
}
