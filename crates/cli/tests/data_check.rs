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
const DAYS: [&str; 3] = ["2024-01-02", "2024-01-03", "2024-01-04"];

/// The universe symbols of the example configuration.
fn universe() -> Vec<String> {
    VALID
        .lines()
        .filter_map(|l| l.strip_prefix("symbol = \""))
        .map(|l| l.trim_end_matches('"').to_string())
        .collect()
}

fn rows(symbol: &str, days: &[&str]) -> String {
    days.iter()
        .map(|d| format!("{symbol},{d},100.0,101.0,99.0,100.5,1000\n"))
        .collect()
}

/// Bars for every universe symbol and BIL on every session, except that the
/// given symbols get only the given days (an empty slice means no bars).
fn bars_except(overrides: &[(&str, &[&str])]) -> String {
    let mut out = String::from(HEADER);
    let mut names = universe();
    names.push("BIL".to_string());
    for name in names {
        let days = overrides
            .iter()
            .find(|(s, _)| *s == name)
            .map_or(&DAYS[..], |(_, d)| *d);
        out.push_str(&rows(&name, days));
    }
    out
}

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

fn lines(out: &Output) -> Vec<String> {
    stdout(out).lines().map(|l| l.trim().to_string()).collect()
}

#[test]
fn data_check_exits_zero_when_there_is_no_gap_and_every_symbol_has_bars() {
    // R-383
    let e = env();
    import(&e, &bars_except(&[]));
    let out = run(&e, &["data", "check"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        !stdout(&out).contains("no-bars"),
        "stdout: {}",
        stdout(&out)
    );
}

#[test]
fn data_check_exits_one_and_lists_symbol_date_line_for_a_gap() {
    // R-383
    let e = env();
    import(&e, &bars_except(&[("SPY", &[DAYS[0], DAYS[2]])]));
    let out = run(&e, &["data", "check"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert_eq!(lines(&out), vec!["SPY 2024-01-03".to_string()]);
}

#[test]
fn data_check_reports_a_trailing_gap() {
    // R-383: the last calendar session is missing.
    let e = env();
    import(&e, &bars_except(&[("SPY", &[DAYS[0], DAYS[1]])]));
    let out = run(&e, &["data", "check"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert_eq!(lines(&out), vec!["SPY 2024-01-04".to_string()]);
}

#[test]
fn data_check_reports_a_gap_in_the_risk_free_symbol() {
    // R-383: BIL is the risk-free symbol.
    let e = env();
    import(&e, &bars_except(&[("BIL", &[DAYS[0], DAYS[2]])]));
    let out = run(&e, &["data", "check"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert_eq!(lines(&out), vec!["BIL 2024-01-03".to_string()]);
}

#[test]
fn data_check_exits_one_with_no_bars_line_for_a_universe_symbol_without_bars() {
    // R-383
    let e = env();
    import(&e, &bars_except(&[("QQQ", &[])]));
    let out = run(&e, &["data", "check"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(lines(&out), vec!["QQQ no-bars".to_string()]);
}
