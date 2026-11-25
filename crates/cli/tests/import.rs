//! R-383: offline imports work in the backtest stage (no owner file, no keys).

mod common;

use chrono::NaiveDate;
use common::{stderr, stdout};
use pt_core::types::{Adjustment, Symbol};
use pt_store::db::Store;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VALID: &str = include_str!("../../../config/paper-trader.example.toml");

struct Env {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    db: PathBuf,
    cfg: PathBuf,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(
        &cfg,
        format!(
            "{VALID}\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            db.display(),
            dir.path().join("reports").display()
        ),
    )
    .unwrap();
    Env { dir, home, db, cfg }
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

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

const CAL: &str = "date\n2024-01-02\n2024-01-03\n2024-01-04\n";
const BARS: &str = "symbol,date,open,high,low,close,volume\n\
    SPY,2024-01-02,472.16,473.67,470.49,472.65,123456\n\
    SPY,2024-01-03,470.43,471.19,468.17,468.79,234567\n";

#[test]
fn calendar_import_exits_zero_prints_session_count_and_stores_sessions() {
    // R-383
    let e = env();
    let csv = write(&e, "cal.csv", CAL);
    let out = run(&e, &["calendar", "import", "--csv", s(&csv)]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).split_whitespace().any(|t| t == "3"),
        "stdout: {}",
        stdout(&out)
    );
    let store = Store::open(&e.db).unwrap();
    let cal = store.calendar().unwrap();
    assert_eq!(cal.sessions_between(d(1), d(31)), &[d(2), d(3), d(4)]);
}

#[test]
fn bars_import_exits_zero_prints_bar_count_and_stores_bars_under_csv_feed() {
    // R-383
    let e = env();
    let csv = write(&e, "bars.csv", BARS);
    let out = run(
        &e,
        &[
            "data",
            "import-csv",
            "--csv",
            s(&csv),
            "--adjustment",
            "all",
            "--source",
            "synth",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).split_whitespace().any(|t| t == "2"),
        "stdout: {}",
        stdout(&out)
    );
    let store = Store::open(&e.db).unwrap();
    let bars = store
        .bars_asof(
            &Symbol::new("SPY").unwrap(),
            Adjustment::All,
            "csv:synth",
            d(1),
            d(31),
            None,
        )
        .unwrap();
    assert_eq!(bars.len(), 2);
}

#[test]
fn raw_adjustment_is_accepted() {
    // R-383
    let e = env();
    let csv = write(&e, "bars.csv", BARS);
    let out = run(
        &e,
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
    let store = Store::open(&e.db).unwrap();
    let raw = store
        .bars_asof(
            &Symbol::new("SPY").unwrap(),
            Adjustment::Raw,
            "csv:synth",
            d(1),
            d(31),
            None,
        )
        .unwrap();
    assert_eq!(raw.len(), 2);
}

#[test]
fn malformed_bars_csv_exits_one_and_stores_nothing() {
    // R-383 (import is atomic, errors exit 1)
    let e = env();
    let bad = format!("{BARS}SPY,2024-01-04,abc,1,1,1,1\n");
    let csv = write(&e, "bad.csv", &bad);
    let out = run(
        &e,
        &[
            "data",
            "import-csv",
            "--csv",
            s(&csv),
            "--adjustment",
            "all",
            "--source",
            "synth",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    assert!(!stderr(&out).is_empty());
    let store = Store::open(&e.db).unwrap();
    let bars = store
        .bars_asof(
            &Symbol::new("SPY").unwrap(),
            Adjustment::All,
            "csv:synth",
            d(1),
            d(31),
            None,
        )
        .unwrap();
    assert!(bars.is_empty());
}

#[test]
fn unknown_adjustment_exits_nonzero() {
    // R-383
    let e = env();
    let csv = write(&e, "bars.csv", BARS);
    let out = run(
        &e,
        &[
            "data",
            "import-csv",
            "--csv",
            s(&csv),
            "--adjustment",
            "bogus",
            "--source",
            "synth",
        ],
    );
    assert_ne!(out.status.code(), Some(0));
    assert_ne!(out.status.code(), None);
}

#[test]
fn missing_csv_file_exits_one() {
    // R-383
    let e = env();
    let missing = e.dir.path().join("nope.csv");
    let out = run(&e, &["calendar", "import", "--csv", s(&missing)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!stderr(&out).is_empty());
}
