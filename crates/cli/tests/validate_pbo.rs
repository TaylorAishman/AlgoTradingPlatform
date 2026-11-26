//! R-391: `validate pbo` prints the Probability of Backtest Overfitting of stored runs.

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
    template: String,
}

impl Env {
    /// Rewrites the config with another `lookback_months`, so the next run is a new trial.
    fn set_lookback(&self, months: u32) {
        let text = self.template.replace("LOOKBACK", &months.to_string());
        std::fs::write(&self.cfg, text).unwrap();
    }
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
         [strategy]\nlookback_months = [LOOKBACK]\nvol_min_obs = 10\nvol_max_obs = 40\n\n\
         [sizing]\nmax_weight = 0.3\n\n\
         [risk]\nmax_order_notional = 1000000\n\n\
         [run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
        dir.path().join("test.sqlite").display(),
        dir.path().join("reports").display()
    )
    .unwrap();
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(&cfg, &text).unwrap();
    let e = Env {
        dir,
        home,
        cfg,
        template: text,
    };
    e.set_lookback(1);

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

fn backtest(e: &Env) {
    let out = run(
        e,
        &[
            "backtest",
            "run",
            "--from",
            "2024-04-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

fn one_run() -> Env {
    let e = env();
    backtest(&e);
    e
}

fn two_runs() -> Env {
    let e = one_run();
    e.set_lookback(2);
    backtest(&e);
    e
}

#[test]
fn validate_pbo_prints_pbo_and_the_number_of_combinations() {
    // R-391: default pbo_partitions = 16, so C(16, 8) = 12870 combinations
    let e = two_runs();
    let out = run(&e, &["validate", "pbo", "--runs", "1,2"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    assert!(text.contains("pbo"), "missing pbo in: {text}");
    assert!(
        text.contains("combinations"),
        "missing combinations in: {text}"
    );
    assert!(
        text.replace([',', '_'], "").contains("12870"),
        "missing 12870 in: {text}"
    );
}

#[test]
fn validate_pbo_with_one_run_exits_one_and_says_it_is_undefined() {
    // R-391 (error case: fewer than 2 trials)
    let e = one_run();
    let out = run(&e, &["validate", "pbo", "--runs", "1"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    let all = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        all.contains("PBO is undefined for fewer than 2 trials"),
        "output: {all}"
    );
}
