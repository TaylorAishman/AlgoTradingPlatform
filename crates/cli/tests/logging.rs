//! R-401: command output goes to stdout, logs go to stderr as JSON lines.

mod common;

use common::{stderr, stdout, trader};
use std::process::{Command, Output};

const VALID: &str = include_str!("../../../config/paper-trader.example.toml");
const FAKE_SECRET: &str = "fake-secret-value-0123456789abcdef";

/// Runs `trader run` at stage `ingest` with fake keys. The cwd and the
/// HOME/owner file are in separate tempdirs.
fn run_command(keys: bool) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let owner = home.path().join("owner.toml");
    std::fs::write(&owner, "stage = \"ingest\"\n").unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.arg("run")
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("PT_OWNER_FILE", &owner)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY");
    if keys {
        cmd.env("APCA_API_KEY_ID", "FAKEKEYID123")
            .env("APCA_API_SECRET_KEY", FAKE_SECRET);
    }
    cmd.output().unwrap()
}

fn assert_json_log_lines(err: &str) {
    let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(!lines.is_empty(), "expected at least one stderr line");
    for line in lines {
        let v: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        for key in ["level", "message", "ts"] {
            assert!(
                v.get(key).and_then(|x| x.as_str()).is_some(),
                "missing string `{key}` in: {line}"
            );
        }
    }
}

#[test]
fn config_check_stdout_holds_only_the_hash_line() {
    // R-401
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cfg.toml");
    std::fs::write(&path, VALID).unwrap();
    let out = trader(&["--config", path.to_str().unwrap(), "config", "check"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {text}");
    assert!(
        lines[0]
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit())),
        "stdout: {text}"
    );
}

#[test]
fn run_stderr_lines_are_json_with_level_message_and_ts() {
    // R-401
    let out = run_command(true);
    assert_json_log_lines(&stderr(&out));
}

#[test]
fn run_failure_without_keys_still_logs_json_lines() {
    // R-401: error reports are log lines too.
    let out = run_command(false);
    assert_eq!(out.status.code(), Some(1));
    assert_json_log_lines(&stderr(&out));
    assert!(stderr(&out).contains("APCA_API_KEY_ID"));
}

#[test]
fn no_stream_contains_the_secret_key_value() {
    // R-401
    let out = run_command(true);
    assert!(!stderr(&out).contains(FAKE_SECRET));
    assert!(!stdout(&out).contains(FAKE_SECRET));
}
