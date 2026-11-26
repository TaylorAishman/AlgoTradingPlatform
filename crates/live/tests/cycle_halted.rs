//! R-315: a halted run settles and reconciles but sends nothing.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::Mode;

fn halted_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .set_halt("manual", "halted for the test", fx.now())
        .expect("halt");
    fx
}

// R-315
#[test]
fn a_run_under_a_manual_halt_ends_halted() {
    let fx = halted_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Halted);
    let row = &fx.store.runs().expect("runs")[0];
    assert_eq!(row.status, "halted");
    assert!(row.finished_at.is_some(), "run row was not finished");
}

// R-315
#[test]
fn a_halted_run_still_settles_and_reconciles() {
    let fx = halted_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert!(outcome.phases.iter().any(|p| p == "settle"), "{outcome:?}");
    assert!(
        outcome.phases.iter().any(|p| p == "reconcile"),
        "{outcome:?}"
    );
    assert!(!outcome.phases.iter().any(|p| p == "submit"), "{outcome:?}");
}

// R-315
#[test]
fn a_halted_run_writes_no_decision() {
    let fx = halted_fixture();

    run_cycle(&fx.ctx()).expect("run");

    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
    assert!(fx
        .store
        .decisions(Mode::DryRun)
        .expect("decisions")
        .is_empty());
}

// R-315
#[test]
fn a_halted_run_sends_no_post_to_the_broker() {
    let fx = halted_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let posts: Vec<_> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert!(posts.is_empty(), "{posts:?}");
}
