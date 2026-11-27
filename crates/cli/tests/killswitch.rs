//! R-396: the kill-switch commands need `--yes`. Without it they exit 1 with a
//! message naming `--yes` and make no request; with it, `cancel-open` cancels
//! the open orders and `flatten` closes the positions and halts.

mod common;

use common::live::{env, posts, run, Env};
use common::{stderr, stdout};

fn deletes(e: &Env) -> Vec<pt_mock_alpaca::state::Recorded> {
    e.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "DELETE")
        .collect()
}

fn assert_refused_without_yes(cmd: &str) {
    let e = env();
    let out = run(&e, &[cmd]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains("--yes"), "message must name --yes: {text}");
    assert!(
        e.mock.requests().is_empty(),
        "no request expected: {:?}",
        e.mock.requests()
    );
}

#[test]
fn cancel_open_without_yes_exits_one_naming_yes_and_makes_no_request() {
    // R-396
    assert_refused_without_yes("cancel-open");
}

#[test]
fn flatten_without_yes_exits_one_naming_yes_and_makes_no_request() {
    // R-396
    assert_refused_without_yes("flatten");
}

#[test]
fn cancel_open_with_yes_prints_two_and_sends_two_deletes() {
    // R-396: after the first paper run two orders are open at the mock.
    let e = env();
    assert_eq!(run(&e, &["run"]).status.code(), Some(0));
    let out = run(&e, &["cancel-open", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(stdout(&out).trim(), "2");
    assert_eq!(deletes(&e).len(), 2, "requests: {:?}", e.mock.requests());
}

#[test]
fn flatten_with_yes_closes_the_position_with_a_flatten_order_and_halts() {
    // R-396 and the architecture table: flatten closes every position and halts.
    let e = env();
    e.mock.seed_position("SPY", 10);
    let out = run(&e, &["flatten", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stdout(&out).split_whitespace().any(|w| w == "1"),
        "stdout should report one order: {}",
        stdout(&out)
    );
    let posts = posts(&e);
    assert_eq!(posts.len(), 1, "POSTs: {posts:?}");
    assert!(posts[0].body.contains("pt-flatten-"), "{posts:?}");
    assert!(posts[0].body.contains("SPY"), "{posts:?}");
    let halt = run(&e, &["halt", "status"]);
    assert_eq!(halt.status.code(), Some(2), "{}", stdout(&halt));
    assert!(stdout(&halt).contains("flattened"), "{}", stdout(&halt));
}

#[test]
fn flatten_with_yes_works_while_a_halt_is_active() {
    // R-396: the kill switch has to work while halted.
    let e = env();
    e.mock.seed_position("SPY", 10);
    assert_eq!(
        run(&e, &["halt", "set", "--reason", "stop"]).status.code(),
        Some(0)
    );
    let out = run(&e, &["flatten", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let posts = posts(&e);
    assert_eq!(posts.len(), 1, "POSTs: {posts:?}");
    assert!(posts[0].body.contains("pt-flatten-"), "{posts:?}");
}
