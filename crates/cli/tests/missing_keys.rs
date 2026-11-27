//! R-386: missing API keys are reported by name and never guessed.

mod common;

use common::{stderr, stdout};
use std::process::{Command, Output};

/// Runs `trader` in an empty directory (the repository root) with an empty
/// HOME and an owner file, in a separate directory, granting `stage`.
fn run(args: &[&str], stage: &str, env: &[(&str, &str)]) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let owner = home.path().join("owner.toml");
    std::fs::write(&owner, format!("stage = \"{stage}\"\n")).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.args(args)
        .current_dir(dir.path())
        .env("HOME", home.path())
        .env("PT_OWNER_FILE", &owner)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn assert_names_missing_keys(args: &[&str], stage: &str) {
    let out = run(args, stage, &[]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{args:?}: stderr: {err}");
    assert!(err.contains("APCA_API_KEY_ID"), "{args:?}: stderr: {err}");
    assert!(err.contains("PT_SECRETS_FILE"), "{args:?}: stderr: {err}");
    assert!(
        stdout(&out).is_empty(),
        "{args:?}: stdout: {}",
        stdout(&out)
    );
}

#[test]
fn data_ingest_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["data", "ingest"], "ingest");
}

#[test]
fn calendar_sync_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["calendar", "sync"], "ingest");
}

#[test]
fn universe_verify_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["universe", "verify"], "ingest");
}

#[test]
fn doctor_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["doctor"], "ingest");
}

#[test]
fn run_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["run"], "ingest");
}

#[test]
fn cancel_open_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["cancel-open", "--yes"], "first_order");
}

#[test]
fn flatten_without_keys_exits_one_naming_variables() {
    // R-386
    assert_names_missing_keys(&["flatten", "--yes"], "paper");
}

#[test]
fn kill_switch_without_keys_exits_one_naming_variables_in_every_stage() {
    // R-396, R-386: no stage forbids the kill switch, so a missing key is the
    // reported failure, not a stage refusal.
    for stage in ["backtest", "ingest", "first_order", "paper"] {
        assert_names_missing_keys(&["cancel-open", "--yes"], stage);
        assert_names_missing_keys(&["flatten", "--yes"], stage);
    }
}

#[test]
fn only_one_key_variable_set_is_still_reported_missing() {
    // R-386: a half-set pair is never completed by guessing.
    let out = run(
        &["data", "ingest"],
        "ingest",
        &[("APCA_API_KEY_ID", "FAKEKEYID")],
    );
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "stderr: {err}");
    assert!(err.contains("APCA_API_SECRET_KEY"), "stderr: {err}");
    assert!(!err.contains("FAKEKEYID"), "key value leaked: {err}");
}

#[test]
fn missing_secrets_file_is_reported_by_name_not_guessed() {
    // R-386
    let out = run(
        &["data", "ingest"],
        "ingest",
        &[("PT_SECRETS_FILE", "/nonexistent/fake-secrets.toml")],
    );
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "stderr: {err}");
    assert!(err.contains("PT_SECRETS_FILE"), "stderr: {err}");
}

#[test]
fn stage_refusal_still_comes_before_key_loading() {
    // R-386 with R-384: a refused command exits 4, not 1, with no keys.
    let out = run(&["data", "ingest"], "backtest", &[]);
    assert_eq!(out.status.code(), Some(4), "stderr: {}", stderr(&out));
}

#[test]
fn offline_command_does_not_ask_for_keys() {
    // R-386: only network commands load keys.
    let out = run(&["gates"], "ingest", &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(!stderr(&out).contains("APCA_API_KEY_ID"));
}
