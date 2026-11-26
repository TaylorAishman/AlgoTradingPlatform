//! R-394 (exit 0 and 1 only): `trader run` reports its outcome by exit code.
//! An ok paper cycle exits 0 and sends the two orders; a failing broker exits
//! 1 and sends nothing. The lock (3) and halted (2) codes are tested elsewhere.

mod common;

use common::live::{env, posts, run};
use common::{stderr, stdout};
use std::path::PathBuf;

#[test]
fn run_in_paper_stage_exits_zero_and_submits_two_orders() {
    // R-394
    let e = env();
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let posts = posts(&e);
    assert_eq!(posts.len(), 2, "POSTs: {posts:?}");
    assert!(posts.iter().all(|p| p.path.ends_with("/v2/orders")));
    assert!(posts[0].body.contains("TLT"), "TLT sell first: {posts:?}");
    assert!(posts[1].body.contains("SPY"), "{posts:?}");
}

#[test]
fn a_second_run_exits_zero_and_sends_no_further_order() {
    // R-394: the run is idempotent, so repeating it is still a success.
    let e = env();
    assert_eq!(run(&e, &["run"]).status.code(), Some(0));
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(posts(&e).len(), 2);
}

#[test]
fn run_exits_one_and_sends_nothing_when_the_broker_fails_every_request() {
    // R-394. A 401 is not retried, so the failure is immediate; a 5xx would
    // make the real client wait out its back-off.
    let e = env();
    e.mock.fail_next(1000, 401);
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(posts(&e).is_empty(), "POSTs: {:?}", posts(&e));
    // The cycle really ran and met the broker's failure: it made a request.
    assert!(
        !e.mock.requests().is_empty(),
        "the run never reached the broker; stderr: {}",
        stderr(&out)
    );
}

fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

#[test]
fn the_calendar_fixture_lives_in_common_live_and_not_in_run_ok() {
    // R-408: `calendar_sessions` is defined once, under tests/common/. The
    // needle is built in pieces so this file does not match its own scan.
    let needle = format!("{} {}", "fn", "calendar_sessions");
    let here = std::fs::read_to_string(tests_dir().join("run_ok.rs")).unwrap();
    let defs_here = here.matches(needle.as_str()).count();
    assert_eq!(defs_here, 0, "run_ok.rs still defines the fixture");
    let live = std::fs::read_to_string(tests_dir().join("common").join("live.rs"))
        .expect("tests/common/live.rs must exist");
    assert_eq!(
        live.matches(needle.as_str()).count(),
        1,
        "common/live.rs must define the fixture exactly once"
    );
}
