//! The offline fixture shared by the `trader` research tests: a temp working
//! directory, a validated config with feed `csv:synth`, and every weekday of
//! 2024 imported from CSV.

use chrono::{Datelike, NaiveDate, Weekday};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::stderr;

pub const SYMBOLS: [&str; 6] = ["S1", "S2", "S3", "S4", "S5", "BIL"];

pub struct Env {
    pub dir: tempfile::TempDir,
    pub home: tempfile::TempDir,
    pub cfg: PathBuf,
}

/// A temp working directory with a validated config (non-empty universe, feed
/// `csv:synth`) and the research fixture imported from CSV: every weekday of 2024.
pub fn env() -> Env {
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

pub fn run(e: &Env, args: &[&str]) -> Output {
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

pub fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

pub fn weekdays() -> Vec<NaiveDate> {
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
pub fn bars_csv(days: &[NaiveDate]) -> String {
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
