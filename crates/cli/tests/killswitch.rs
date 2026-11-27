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

/// Runs the command in `stage` with an owner file, optionally without keys.
fn run_in_stage(e: &Env, stage: &str, keys: bool, args: &[&str]) -> std::process::Output {
    std::fs::write(&e.owner, format!("stage = \"{stage}\"\n")).unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.arg("--config")
        .arg(&e.cfg)
        .args(args)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("PT_OWNER_FILE", &e.owner)
        .env("PT_ALPACA_TRADING_URL", e.mock.url())
        .env("PT_ALPACA_DATA_URL", e.mock.url())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY");
    if keys {
        cmd.env("APCA_API_KEY_ID", "fake-key-id")
            .env("APCA_API_SECRET_KEY", "fake-secret");
    }
    cmd.output().unwrap()
}

#[test]
fn without_yes_exits_one_naming_yes_in_every_stage_with_or_without_keys() {
    // R-396: confirmation comes before the stage check and the key check.
    for cmd in ["cancel-open", "flatten"] {
        for stage in ["backtest", "paper"] {
            for keys in [true, false] {
                let e = env();
                let out = run_in_stage(&e, stage, keys, &[cmd]);
                let text = format!("{}{}", stdout(&out), stderr(&out));
                assert_eq!(out.status.code(), Some(1), "{cmd} {stage} {keys}: {text}");
                assert!(text.contains("--yes"), "{cmd} {stage} {keys}: {text}");
                assert!(e.mock.requests().is_empty(), "{cmd} {stage} {keys}");
            }
        }
    }
}

#[test]
fn with_yes_and_no_keys_exits_one_naming_missing_keys_and_makes_no_request() {
    // R-396
    for cmd in ["cancel-open", "flatten"] {
        let e = env();
        let out = run_in_stage(&e, "paper", false, &[cmd, "--yes"]);
        let err = stderr(&out);
        assert_eq!(out.status.code(), Some(1), "{cmd}: {err}");
        assert!(err.contains("APCA_API_KEY_ID"), "{cmd}: {err}");
        assert!(e.mock.requests().is_empty(), "{cmd}");
    }
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
