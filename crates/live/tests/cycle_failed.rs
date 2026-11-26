//! R-316: a broker or data failure fails the run cleanly.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::Mode;

fn failing_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.fail_next(100, 500);
    fx
}

// R-316
#[test]
fn a_broker_failure_ends_the_run_failed_instead_of_returning_an_error() {
    let fx = failing_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("a failed run is still a result");

    assert_eq!(outcome.status, RunStatus::Failed);
}

// R-316
#[test]
fn a_failed_run_is_finished_in_the_run_log() {
    let fx = failing_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let rows = fx.store.runs().expect("runs");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "failed");
    assert!(rows[0].finished_at.is_some(), "run row was not finished");
}

// R-316
#[test]
fn a_failed_run_records_a_broker_or_feed_alert() {
    let fx = failing_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let alerts = fx.store.alerts().expect("alerts");
    assert!(
        alerts
            .iter()
            .any(|a| a.1 == "broker_error" || a.1 == "feed_error"),
        "{alerts:?}"
    );
}

// R-316
#[test]
fn a_failed_run_writes_no_decision_and_no_order() {
    let fx = failing_fixture();

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

// R-316
#[test]
fn a_failed_run_sends_no_post_to_the_broker() {
    let fx = failing_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let posts: Vec<_> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert!(posts.is_empty(), "{posts:?}");
}
