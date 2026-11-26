//! R-312: a run in the backtest stage must not touch the network.

mod common;

use common::LiveFixture;
use pt_gate::error::GateError;
use pt_gate::owner::Stage;
use pt_live::cycle::run_cycle;
use pt_live::error::LiveError;

// R-312
#[test]
fn in_the_backtest_stage_run_cycle_is_refused_and_sends_no_request() {
    let fx = LiveFixture::new(Stage::Backtest);

    let result = run_cycle(&fx.ctx());

    assert!(
        matches!(result, Err(LiveError::Gate(GateError::StageForbids))),
        "got {result:?}"
    );
    assert!(
        fx.mock.requests().is_empty(),
        "the mock recorded requests: {:?}",
        fx.mock.requests()
    );
}
