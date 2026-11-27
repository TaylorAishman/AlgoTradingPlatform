//! R-384: a command the stage forbids exits 4, naming the required stage,
//! before any key is loaded or request made.

mod common;

use common::{stderr, stdout};
use std::process::{Command, Output};

/// Runs `trader` in an empty directory with an empty HOME, no keys, and an
/// optional owner file setting the stage.
fn run(args: &[&str], stage: Option<&str>) -> Output {
    // The cwd is the repository root; HOME and the owner file live in a
    // separate directory so the owner file is outside the repository.
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.args(args)
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("PT_OWNER_FILE");
    if let Some(s) = stage {
        let owner = home.path().join("owner.toml");
        std::fs::write(&owner, format!("stage = \"{s}\"\n")).unwrap();
        cmd.env("PT_OWNER_FILE", &owner);
    }
    cmd.output().unwrap()
}

fn assert_refused(args: &[&str], stage: Option<&str>, required: &str) {
    let out = run(args, stage);
    assert_eq!(
        out.status.code(),
        Some(4),
        "{args:?}: stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stderr(&out).contains(required),
        "{args:?}: stderr should name `{required}`: {}",
        stderr(&out)
    );
}

#[test]
fn data_ingest_in_backtest_stage_exits_four_naming_ingest() {
    // R-384
    assert_refused(&["data", "ingest"], None, "ingest");
}

#[test]
fn calendar_sync_in_backtest_stage_exits_four_naming_ingest() {
    // R-384
    assert_refused(&["calendar", "sync"], None, "ingest");
}

#[test]
fn universe_verify_in_backtest_stage_exits_four_naming_ingest() {
    // R-384
    assert_refused(&["universe", "verify"], None, "ingest");
}

#[test]
fn doctor_in_backtest_stage_exits_four_naming_ingest() {
    // R-384
    assert_refused(&["doctor"], None, "ingest");
}

#[test]
fn run_in_backtest_stage_exits_four_naming_ingest() {
    // R-384
    assert_refused(&["run"], None, "ingest");
}

/// Runs a kill-switch command in the live fixture (config, seeded mock and
/// clock, fake keys) but with no owner file, so the stage is `backtest`.
fn run_kill_without_owner(cmd: &str) -> (Output, common::live::Env) {
    let e = common::live::env();
    let out = Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args([cmd, "--yes"])
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("APCA_API_KEY_ID", "fake-key-id")
        .env("APCA_API_SECRET_KEY", "fake-secret")
        .env("PT_ALPACA_TRADING_URL", e.mock.url())
        .env("PT_ALPACA_DATA_URL", e.mock.url())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("PT_OWNER_FILE")
        .output()
        .unwrap();
    (out, e)
}

#[test]
fn cancel_open_with_yes_and_no_owner_file_exits_zero() {
    // R-384: the kill switch is forbidden in no stage.
    let (out, _e) = run_kill_without_owner("cancel-open");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

#[test]
fn flatten_with_yes_and_no_owner_file_exits_zero() {
    // R-384: the kill switch is forbidden in no stage.
    let (out, _e) = run_kill_without_owner("flatten");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
}

#[test]
fn refusal_prints_nothing_on_stdout() {
    // R-384: the refused command did no work.
    let out = run(&["data", "ingest"], None);
    assert_eq!(out.status.code(), Some(4));
    assert!(stdout(&out).is_empty(), "stdout: {}", stdout(&out));
}
