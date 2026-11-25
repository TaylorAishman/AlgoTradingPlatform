//! R-387: `trader universe verify` prints one line per universe symbol with
//! PASS or FAIL and reasons, and signals failure by exit code.

mod common;

use common::{stderr, stdout};
use pt_mock_alpaca::server::MockAlpaca;
use std::path::PathBuf;
use std::process::{Command, Output};

use chrono::{Datelike, NaiveDate, Weekday};

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, m, day).unwrap()
}

/// Weekdays from 2024-01-02 to 2024-02-29 less the two holidays: 41 sessions.
fn sessions() -> Vec<NaiveDate> {
    let holidays = [d(1, 15), d(2, 19)];
    let mut out = Vec::new();
    let mut day = d(1, 2);
    while day <= d(2, 29) {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) && !holidays.contains(&day) {
            out.push(day);
        }
        day = day.succ_opt().unwrap();
    }
    out
}

const CONFIG: &str = r#"
[data]
start = "2024-01-02"

[strategy]
lookback_months = [1]
risk_free_symbol = "BIL"
vol_min_obs = 5
vol_max_obs = 20

[sizing]
max_weight = 0.5

[risk]
max_order_notional = 60000

[[universe]]
symbol = "SPY"
asset_class = "equity"
shortable = true

[[universe]]
symbol = "TLT"
asset_class = "equity"
shortable = true
"#;

struct Env {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    cfg: PathBuf,
    owner: PathBuf,
    mock: MockAlpaca,
}

fn env(stage: &str) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(
        &cfg,
        format!(
            "{CONFIG}\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            db.display(),
            dir.path().join("reports").display()
        ),
    )
    .unwrap();
    let owner = home.path().join("owner.toml");
    std::fs::write(&owner, format!("stage = \"{stage}\"\n")).unwrap();

    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    let days = sessions();
    let dates: Vec<String> = days.iter().map(|s| s.to_string()).collect();
    let refs: Vec<&str> = dates.iter().map(String::as_str).collect();
    mock.seed_sessions(&refs);
    for s in ["SPY", "TLT", "BIL"] {
        mock.seed_asset(s, true, true, true);
    }
    mock.set_clock(
        "2024-03-01T08:00:00-05:00",
        false,
        "2024-03-01T09:30:00-05:00",
        "2024-03-01T16:00:00-05:00",
    );
    let mut bars = Vec::new();
    for (i, date) in days.iter().enumerate() {
        let i = i as i64;
        let closes = [
            (100 + i + (i % 2)) as f64,
            (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0,
            (10000 + i) as f64 / 100.0,
        ];
        for (k, name) in ["SPY", "TLT", "BIL"].iter().enumerate() {
            let close = closes[k];
            bars.push((
                name.to_string(),
                date.to_string(),
                close,
                close,
                close,
                close,
                1_000_000u64,
            ));
        }
    }
    mock.seed_bars("raw", "sip", bars.clone());
    mock.seed_bars("all", "sip", bars);
    Env {
        dir,
        home,
        cfg,
        owner,
        mock,
    }
}

fn run(e: &Env, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args(args)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("PT_OWNER_FILE", &e.owner)
        .env("APCA_API_KEY_ID", "fake-key-id")
        .env("APCA_API_SECRET_KEY", "fake-secret")
        .env("PT_ALPACA_TRADING_URL", e.mock.url())
        .env("PT_ALPACA_DATA_URL", e.mock.url())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .output()
        .unwrap()
}

fn ok(out: &Output) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(out),
        stderr(out)
    );
}

fn prepared(stage: &str) -> Env {
    let e = env(stage);
    ok(&run(&e, &["calendar", "sync"]));
    ok(&run(&e, &["data", "ingest"]));
    e
}

fn line_for<'a>(text: &'a str, symbol: &str) -> Vec<&'a str> {
    text.lines()
        .filter(|l| {
            l.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|t| t == symbol)
        })
        .collect()
}

#[test]
fn verify_prints_one_pass_line_per_symbol_and_exits_zero_when_all_pass() {
    // R-387
    let e = prepared("ingest");
    let out = run(&e, &["universe", "verify"]);
    ok(&out);
    let text = stdout(&out);
    for symbol in ["SPY", "TLT"] {
        let lines = line_for(&text, symbol);
        assert_eq!(lines.len(), 1, "one line for {symbol} in: {text}");
        assert!(lines[0].contains("PASS"), "{}", lines[0]);
        assert!(!lines[0].contains("FAIL"), "{}", lines[0]);
    }
}

#[test]
fn verify_marks_a_failing_symbol_with_its_reason_and_exits_one() {
    // R-387
    let e = prepared("ingest");
    e.mock.seed_asset("TLT", true, false, false);
    let out = run(&e, &["universe", "verify"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let text = stdout(&out);
    let tlt = line_for(&text, "TLT");
    assert_eq!(tlt.len(), 1, "one line for TLT in: {text}");
    assert!(tlt[0].contains("FAIL"), "{}", tlt[0]);
    assert!(tlt[0].contains("not_shortable"), "{}", tlt[0]);
    let spy = line_for(&text, "SPY");
    assert_eq!(spy.len(), 1, "one line for SPY in: {text}");
    assert!(spy[0].contains("PASS"), "{}", spy[0]);
}
