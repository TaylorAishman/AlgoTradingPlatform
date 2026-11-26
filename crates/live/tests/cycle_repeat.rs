//! R-314: repeating a paper-stage run with the same clock changes nothing.

mod common;

use common::LiveFixture;
use pt_core::order_state::OrderState;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::Mode;

fn post_count(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .count()
}

// R-314
#[test]
fn a_second_run_with_the_same_clock_is_ok_and_sends_nothing_new() {
    let fx = LiveFixture::new(Stage::Paper);
    run_cycle(&fx.ctx()).expect("first run");
    let posts_after_first = post_count(&fx);
    assert_eq!(posts_after_first, 2);

    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(fx.store.decisions(Mode::Paper).expect("decisions").len(), 1);
    let submitted = fx
        .store
        .orders_in_states(&[OrderState::Submitted])
        .expect("orders");
    assert_eq!(submitted.len(), 2, "{submitted:?}");
    assert_eq!(post_count(&fx), posts_after_first, "second run posted");
}
