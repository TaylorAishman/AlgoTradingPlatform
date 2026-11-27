//! R-501, R-502: the first-order stage never rebalances, and a halt stops its order.

mod common;

use common::{d, LiveFixture};
use pt_core::order_state::OrderState;
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

fn assert_no_rebalance(fx: &LiveFixture) {
    let outcome = run_cycle(&fx.ctx()).expect("run");
    assert!(
        !outcome.phases.iter().any(|p| p == "rebalance"),
        "{outcome:?}"
    );
    for mode in [Mode::Paper, Mode::DryRun] {
        assert!(
            fx.store.decisions(mode).expect("decisions").is_empty(),
            "decision row written in {mode:?}"
        );
    }
}

// R-501
#[test]
fn a_first_order_run_with_an_approval_never_rebalances() {
    assert_no_rebalance(&approved_fixture());
}

// R-501
#[test]
fn a_first_order_run_without_an_approval_never_rebalances() {
    assert_no_rebalance(&LiveFixture::new(Stage::FirstOrder));
}

// R-502
#[test]
fn a_halted_first_order_run_ends_halted_and_sends_no_post() {
    let fx = approved_fixture();
    fx.store
        .set_halt("manual", "halted for the test", fx.now())
        .expect("halt");

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Halted);
    let posts: Vec<_> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert!(posts.is_empty(), "{posts:?}");
    let submitted = fx
        .store
        .orders_in_states(&[OrderState::Submitted])
        .expect("orders");
    assert!(submitted.is_empty(), "{submitted:?}");
}
