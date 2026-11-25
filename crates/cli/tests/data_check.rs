//! R-383: `trader data check` works offline in the backtest stage. It exits 0
//! when there is no gap and exits 1 listing `SYMBOL DATE` lines when there is one.

mod common;

use common::{stderr, stdout};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VALID: &str = include_str!("../../../config/paper-trader.example.toml");

struct Env {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    cfg: PathBuf,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");
    let cfg = dir.path().join("cfg.toml");
    let base = VALID.replace("feed = \"sip\"", "feed = \"csv:synth\"");
    assert!(base.contains("csv:synth"));
    std::fs::write(
        &cfg,
        format!(
            "{base}\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            db.display(),
            dir.path().join("reports").display()
        ),
    )
    .unwrap();
    Env { dir, home, cfg }
}

fn run(e: &Env, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args(args)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env_remove("PT_CONFIG")
        .env_remove("PT_OWNER_FILE")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .output()
        .unwrap()
}

fn write(e: &Env, name: &str, text: &str) -> PathBuf {
    let p = e.dir.path().join(name);
    std::fs::write(&p, text).unwrap();
    p
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

const CAL: &str = "date\n2024-01-02\n2024-01-03\n2024-01-04\n";
const HEADER: &str = "symbol,date,open,high,low,close,volume\n";
const DAY2: &str = "SPY,2024-01-02,472.16,473.67,470.49,472.65,123456\n";
const DAY3: &str = "SPY,2024-01-03,470.43,471.19,468.17,468.79,234567\n";
const DAY4: &str = "SPY,2024-01-04,468.0,469.5,466.1,467.2,345678\n";

/// Imports the calendar and the raw bars under the `csv:synth` feed.
fn import(e: &Env, bars: &str) {
    let cal = write(e, "cal.csv", CAL);
    let out = run(e, &["calendar", "import", "--csv", s(&cal)]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let csv = write(e, "bars.csv", bars);
    let out = run(
        e,
        &[
            "data",
            "import-csv",
            "--csv",
            s(&csv),
            "--adjustment",
            "raw",
            "--source",
            "synth",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

#[test]
fn data_check_exits_zero_when_there_is_no_gap() {
    // R-383
    let e = env();
    import(&e, &format!("{HEADER}{DAY2}{DAY3}{DAY4}"));
    let out = run(&e, &["data", "check"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        !stdout(&out).contains("SPY 2024-"),
        "no gap line expected, stdout: {}",
        stdout(&out)
    );
}

#[test]
fn data_check_exits_one_and_lists_symbol_date_line_for_a_gap() {
    // R-383
    let e = env();
    import(&e, &format!("{HEADER}{DAY2}{DAY4}"));
    let out = run(&e, &["data", "check"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let lines: Vec<String> = stdout(&out).lines().map(|l| l.trim().to_string()).collect();
    assert!(
        lines.iter().any(|l| l == "SPY 2024-01-03"),
        "expected a `SPY 2024-01-03` line, stdout: {}",
        stdout(&out)
    );
    assert!(
        !lines
            .iter()
            .any(|l| l == "SPY 2024-01-02" || l == "SPY 2024-01-04"),
        "only the missing session is listed, stdout: {}",
        stdout(&out)
    );
}

#[test]
fn data_check_reports_a_trailing_gap() {
    // R-383: the last calendar session is missing.
    let e = env();
    import(&e, &format!("{HEADER}{DAY2}{DAY3}"));
    let out = run(&e, &["data", "check"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let lines: Vec<String> = stdout(&out).lines().map(|l| l.trim().to_string()).collect();
    assert_eq!(lines, vec!["SPY 2024-01-04".to_string()]);
}

#[test]
fn data_check_reports_a_gap_in_the_risk_free_symbol() {
    // R-383: BIL is the risk-free symbol.
    let e = env();
    let bil = |d: &str| format!("BIL,{d},91.0,91.2,90.9,91.1,1000\n");
    import(
        &e,
        &format!(
            "{HEADER}{DAY2}{DAY3}{DAY4}{}{}",
            bil("2024-01-02"),
            bil("2024-01-04")
        ),
    );
    let out = run(&e, &["data", "check"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    let lines: Vec<String> = stdout(&out).lines().map(|l| l.trim().to_string()).collect();
    assert_eq!(lines, vec!["BIL 2024-01-03".to_string()]);
}
