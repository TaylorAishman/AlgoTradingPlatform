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

/// Runs `trader --config <cfg> gates` with only the given extra variables.
fn gates_with(
    cfg: &std::path::Path,
    home: &std::path::Path,
    vars: &[(&str, &std::path::Path)],
) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.arg("--config")
        .arg(cfg)
        .arg("gates")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("HOME", home)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("PT_OWNER_FILE");
    for (k, v) in vars {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

/// A config with one symbol whose store lives in `dir`.
fn write_config(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("cfg.toml");
    std::fs::write(
        &cfg,
        format!(
            "[[universe]]\nsymbol = \"S1\"\nasset_class = \"equity\"\nshortable = true\n\n\
             [data]\nfeed = \"csv:synth\"\n\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            dir.join("store.sqlite").display(),
            dir.join("reports").display()
        ),
    )
    .unwrap();
    cfg
}

fn trader_in(
    dir: &std::path::Path,
    home: &std::path::Path,
    cfg: &std::path::Path,
    args: &[&str],
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(cfg)
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("PT_OWNER_FILE")
        .output()
        .unwrap()
}

#[test]
fn keys_from_a_secrets_file_mark_the_keys_line_done() {
    // G-23: `PT_SECRETS_FILE` is accepted by every command that loads keys,
    // so the checklist must count it too.
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let keys = dir.path().join("keys.toml");
    std::fs::write(
        &keys,
        "key_id = \"fake-id\"\nsecret_key = \"fake-secret\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&keys, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let out = gates_with(&cfg, home.path(), &[("PT_SECRETS_FILE", &keys)]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    let line = line_for(&text, "api_keys");
    assert!(line.contains("done"), "line: {line}");
    assert!(!text.contains("fake-secret") && !stderr(&out).contains("fake-secret"));
}

#[test]
fn a_store_with_a_calendar_and_bars_marks_both_lines_done() {
    // The checklist reads the store named by the config, offline.
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let cal = dir.path().join("cal.csv");
    std::fs::write(&cal, "date\n2024-01-02\n2024-01-03\n").unwrap();
    let bars = dir.path().join("bars.csv");
    std::fs::write(
        &bars,
        "symbol,date,open,high,low,close,volume\n\
         S1,2024-01-02,10.00,10.50,9.90,10.20,1000\n\
         S1,2024-01-03,10.20,10.40,10.00,10.30,1000\n",
    )
    .unwrap();
    for args in [
        vec!["calendar", "import", "--csv", cal.to_str().unwrap()],
        vec![
            "data",
            "import-csv",
            "--csv",
            bars.to_str().unwrap(),
            "--adjustment",
            "raw",
            "--source",
            "synth",
        ],
    ] {
        let out = trader_in(dir.path(), home.path(), &cfg, &args);
        assert_eq!(out.status.code(), Some(0), "{args:?}: {}", stderr(&out));
    }
    let out = gates_with(&cfg, home.path(), &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    for id in ["calendar", "bars"] {
        let line = line_for(&text, &format!("{id}:"));
        assert!(line.contains("done"), "line: {line}");
    }
}

#[test]
fn a_config_whose_store_does_not_exist_leaves_both_pending_and_creates_nothing() {
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let out = gates_with(&cfg, home.path(), &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    for id in ["calendar", "bars"] {
        let line = line_for(&text, &format!("{id}:"));
        assert!(line.contains("pending"), "line: {line}");
    }
    assert!(
        !dir.path().join("store.sqlite").exists(),
        "gates created the store"
    );
}
