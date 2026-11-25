//! R-388: a research backtest runs from the command line, offline, and reports its
//! metrics, the Deflated Sharpe Ratio and the trial count.

mod common;

use chrono::{Datelike, NaiveDate, Weekday};
use common::{stderr, stdout};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SYMBOLS: [&str; 6] = ["S1", "S2", "S3", "S4", "S5", "BIL"];

struct Env {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    cfg: PathBuf,
}

/// A temp working directory with a validated config (non-empty universe, feed
/// `csv:synth`) and the research fixture imported from CSV: every weekday of 2024.
fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut text = String::new();
    for s in &SYMBOLS[..5] {
        write!(
            text,
            "[[universe]]\nsymbol = \"{s}\"\nasset_class = \"equity\"\nshortable = true\n\n"
        )
        .unwrap();
    }
    write!(
        text,
        "[data]\nfeed = \"csv:synth\"\n\n\
         [strategy]\nlookback_months = [1]\nvol_min_obs = 10\nvol_max_obs = 40\n\n\
         [sizing]\nmax_weight = 0.3\n\n\
         [risk]\nmax_order_notional = 1000000\n\n\
         [run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
        dir.path().join("test.sqlite").display(),
        dir.path().join("reports").display()
    )
    .unwrap();
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(&cfg, text).unwrap();
    let e = Env { dir, home, cfg };

    let days = weekdays();
    let mut cal = String::from("date\n");
    for d in &days {
        writeln!(cal, "{d}").unwrap();
    }
    let cal_path = e.dir.path().join("cal.csv");
    std::fs::write(&cal_path, cal).unwrap();
    let out = run(&e, &["calendar", "import", "--csv", s(&cal_path)]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let bars_path = e.dir.path().join("bars.csv");
    std::fs::write(&bars_path, bars_csv(&days)).unwrap();
    for adj in ["all", "raw"] {
        let out = run(
            &e,
            &[
                "data",
                "import-csv",
                "--csv",
                s(&bars_path),
                "--adjustment",
                adj,
                "--source",
                "synth",
            ],
        );
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    }
    e
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

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn weekdays() -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut d = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    while d.year() == 2024 {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(d);
        }
        d = d.succ_opt().unwrap();
    }
    out
}

/// Bars for every symbol and weekday; prices in integer cents from a fixed-seed
/// linear congruential generator (daily moves of -2% to +2%, BIL +0.01%).
fn bars_csv(days: &[NaiveDate]) -> String {
    let mut out = String::from("symbol,date,open,high,low,close,volume\n");
    for (i, sym) in SYMBOLS.iter().enumerate() {
        let mut state: u64 = 1_000 + i as u64;
        let mut prev: i64 = 10_000;
        for d in days {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let bp: i64 = if *sym == "BIL" {
                1
            } else {
                ((state >> 33) % 4001) as i64 - 2000
            };
            let close = (prev * (10_000 + bp) + 5_000) / 10_000;
            let hi = prev.max(close);
            let lo = prev.min(close);
            writeln!(
                out,
                "{sym},{d},{}.{:02},{}.{:02},{}.{:02},{}.{:02},1000000",
                prev / 100,
                prev % 100,
                hi / 100,
                hi % 100,
                lo / 100,
                lo % 100,
                close / 100,
                close % 100
            )
            .unwrap();
            prev = close;
        }
    }
    out
}

/// Lowercased stdout with `=` and `:` turned into spaces, split into tokens.
fn tokens(out: &Output) -> Vec<String> {
    stdout(out)
        .to_lowercase()
        .replace(['=', ':'], " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn has_pair(toks: &[String], key: &str, value: &str) -> bool {
    toks.windows(2).any(|w| w[0] == key && w[1] == value)
}

fn trial_hash(toks: &[String]) -> Option<String> {
    toks.iter()
        .find(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))
        .cloned()
}

#[test]
fn backtest_run_exits_zero_and_prints_metrics_dsr_and_one_trial() {
    // R-388
    let e = env();
    let out = run(
        &e,
        &[
            "backtest",
            "run",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    for label in [
        "run", "sharpe", "cagr", "vol", "drawdown", "turnover", "cost", "borrow", "dsr",
    ] {
        assert!(text.contains(label), "missing {label:?} in: {text}");
    }
    let toks = tokens(&out);
    assert!(
        trial_hash(&toks).is_some(),
        "no 64-hex trial hash in: {text}"
    );
    assert!(has_pair(&toks, "trials", "1"), "no `trials 1` in: {text}");
}

#[test]
fn backtest_run_prints_the_first_run_id() {
    // R-388
    let e = env();
    let out = run(
        &e,
        &[
            "backtest",
            "run",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let toks = tokens(&out);
    assert!(
        has_pair(&toks, "run", "1") || has_pair(&toks, "id", "1"),
        "no run id 1 in: {}",
        stdout(&out)
    );
}

#[test]
fn benchmark_strategy_runs_and_exits_zero() {
    // R-388
    let e = env();
    let out = run(
        &e,
        &[
            "backtest",
            "run",
            "--strategy",
            "benchmark",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    assert!(text.contains("sharpe"), "stdout: {text}");
    assert!(has_pair(&tokens(&out), "trials", "1"), "stdout: {text}");
}

#[test]
fn second_identical_run_prints_the_same_hash_and_still_one_trial() {
    // R-388
    let e = env();
    let args = [
        "backtest",
        "run",
        "--from",
        "2024-03-01",
        "--to",
        "2024-12-30",
    ];
    let first = run(&e, &args);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    let second = run(&e, &args);
    assert_eq!(second.status.code(), Some(0), "stderr: {}", stderr(&second));
    let (t1, t2) = (tokens(&first), tokens(&second));
    assert!(trial_hash(&t1).is_some(), "stdout: {}", stdout(&first));
    assert_eq!(trial_hash(&t1), trial_hash(&t2));
    assert!(
        has_pair(&t2, "trials", "1"),
        "no `trials 1` in: {}",
        stdout(&second)
    );
}
