//! R-393: the offline path works end to end with no owner file, no keys and no network.

mod common;

use chrono::{Datelike, NaiveDate, Weekday};
use common::{stderr, stdout};
use pt_mock_alpaca::server::MockAlpaca;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SYMBOLS: [&str; 6] = ["S1", "S2", "S3", "S4", "S5", "BIL"];

fn run(dir: &Path, home: &Path, cfg: &Path, mock: &MockAlpaca, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(cfg)
        .args(args)
        .current_dir(dir)
        .env("HOME", home)
        .env_remove("PT_CONFIG")
        .env_remove("PT_OWNER_FILE")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("APCA_API_BASE_URL")
        .env_remove("APCA_API_DATA_URL")
        .env("PT_ALPACA_TRADING_URL", mock.url())
        .env("PT_ALPACA_DATA_URL", mock.url())
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

#[test]
fn offline_research_path_exits_zero_at_every_step_and_writes_status_json() {
    // R-393
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mock = MockAlpaca::start();
    let mut text = String::new();
    for sym in &SYMBOLS[..5] {
        write!(
            text,
            "[[universe]]\nsymbol = \"{sym}\"\nasset_class = \"equity\"\nshortable = true\n\n"
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
    let cfg: PathBuf = dir.path().join("cfg.toml");
    std::fs::write(&cfg, text).unwrap();

    let days = weekdays();
    let mut cal = String::from("date\n");
    for d in &days {
        writeln!(cal, "{d}").unwrap();
    }
    let cal_path = dir.path().join("cal.csv");
    std::fs::write(&cal_path, cal).unwrap();
    let bars_path = dir.path().join("bars.csv");
    std::fs::write(&bars_path, bars_csv(&days)).unwrap();
    let (cal_s, bars_s) = (s(&cal_path), s(&bars_path));

    let steps: Vec<Vec<&str>> = vec![
        vec!["calendar", "import", "--csv", cal_s],
        vec![
            "data",
            "import-csv",
            "--csv",
            bars_s,
            "--adjustment",
            "all",
            "--source",
            "synth",
        ],
        vec![
            "data",
            "import-csv",
            "--csv",
            bars_s,
            "--adjustment",
            "raw",
            "--source",
            "synth",
        ],
        vec!["data", "check"],
        vec![
            "backtest",
            "run",
            "--from",
            "2024-04-01",
            "--to",
            "2024-12-30",
        ],
        vec![
            "backtest",
            "crosscheck",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
        vec!["validate", "dsr", "--run", "1"],
        vec!["report"],
    ];
    for args in &steps {
        let out = run(dir.path(), home.path(), &cfg, &mock, args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "step {args:?}\nstdout: {}\nstderr: {}",
            stdout(&out),
            stderr(&out)
        );
    }

    // No network request was made, and no owner file was written at the
    // default path under the temp HOME.
    assert!(
        mock.requests().is_empty(),
        "requests: {:?}",
        mock.requests()
    );
    let default_owner = home.path().join(".config/paper-trader/owner.toml");
    assert!(!default_owner.exists(), "owner file at {default_owner:?}");

    let status = dir.path().join("reports").join("status.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&status).expect("status.json exists"))
            .expect("status.json is JSON");
    assert_eq!(json["trial_count"], 1, "{json}");
}
