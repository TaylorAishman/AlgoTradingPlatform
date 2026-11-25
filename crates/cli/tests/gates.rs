//! R-382: the stage and the checklist are visible without keys or network.

mod common;

use common::{stderr, stdout, trader};
use std::process::{Command, Output};

/// Runs `trader gates` with a scrubbed environment, an empty HOME (so the
/// default owner file cannot exist) and the given owner-file variable.
fn gates(owner_file: Option<&std::path::Path>, home: &std::path::Path) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.arg("gates")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("HOME", home)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("PT_OWNER_FILE");
    if let Some(p) = owner_file {
        cmd.env("PT_OWNER_FILE", p);
    }
    cmd.output().unwrap()
}

fn checklist_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.contains("pending") || l.contains("done"))
        .collect()
}

fn line_for<'a>(text: &'a str, id: &str) -> &'a str {
    text.lines()
        .find(|l| l.contains(id))
        .unwrap_or_else(|| panic!("no line for {id} in: {text}"))
}

#[test]
fn no_owner_file_and_no_keys_prints_backtest_and_twelve_pending_lines() {
    // R-382
    let home = tempfile::tempdir().unwrap();
    let out = gates(None, home.path());
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("stage: backtest"), "stdout: {text}");
    let lines = checklist_lines(&text);
    assert_eq!(lines.len(), 12, "stdout: {text}");
    assert!(
        lines.iter().all(|l| l.contains("pending")),
        "stdout: {text}"
    );
}

#[test]
fn missing_owner_file_at_pt_owner_file_means_stage_backtest() {
    // R-382
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("owner.toml");
    let out = gates(Some(&missing), home.path());
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("stage: backtest"));
}

#[test]
fn owner_file_with_dry_run_marks_ingest_and_dry_run_lines_done() {
    // R-382
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let owner = dir.path().join("owner.toml");
    std::fs::write(&owner, "stage = \"dry_run\"\n").unwrap();
    let out = gates(Some(&owner), home.path());
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("stage: dry_run"), "stdout: {text}");
    for id in ["stage_ingest", "stage_dry_run"] {
        let line = line_for(&text, id);
        assert!(line.contains("done"), "line: {line}");
        assert!(!line.contains("pending"), "line: {line}");
    }
    let later = line_for(&text, "stage_paper");
    assert!(later.contains("pending"), "line: {later}");
}

#[test]
fn owner_file_inside_the_repository_exits_one_saying_outside_the_repository() {
    // R-382
    let home = tempfile::tempdir().unwrap();
    // A file inside the crate directory, which is inside the repository.
    let inside = tempfile::NamedTempFile::new_in(env!("CARGO_MANIFEST_DIR")).unwrap();
    std::fs::write(inside.path(), "stage = \"paper\"\n").unwrap();
    let out = gates(Some(inside.path()), home.path());
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    assert!(
        stderr(&out).contains("outside the repository"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn gates_is_listed_by_the_shared_helper_environment_without_keys() {
    // R-382: the plain helper (no owner file, no keys) also succeeds.
    let out = trader(&["gates"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}
