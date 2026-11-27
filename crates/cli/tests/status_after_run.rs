//! R-540: the heartbeat check passes after a real run, with no run row
//! written by the test itself.

mod common;

use common::live::{env, run};
use common::{stderr, stdout};

#[test]
fn status_fails_before_any_run_and_passes_straight_after_a_real_run() {
    // R-540
    let e = env();
    let before = run(&e, &["status", "--max-age-hours", "24"]);
    assert_eq!(
        before.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&before),
        stderr(&before)
    );

    let ran = run(&e, &["run"]);
    assert_eq!(
        ran.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&ran),
        stderr(&ran)
    );

    let after = run(&e, &["status", "--max-age-hours", "24"]);
    assert_eq!(
        after.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&after),
        stderr(&after)
    );
}
