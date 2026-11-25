//! R-385: calendar sync and data ingest run from the command line once the
//! stage allows, against the mock broker, and leave data that checks clean.

mod common;

use common::{stderr, stdout};
use pt_core::types::{Adjustment, Symbol};
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;
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
    db: PathBuf,
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
        db,
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

#[test]
fn calendar_sync_exits_zero_and_stores_the_broker_sessions() {
    // R-385
    let e = env("ingest");
    let out = run(&e, &["calendar", "sync"]);
    ok(&out);
    let store = Store::open(&e.db).unwrap();
    let cal = store.calendar().unwrap();
    assert_eq!(cal.sessions_between(d(1, 1), d(3, 31)), &sessions()[..]);
}

#[test]
fn ingest_prints_counts_for_both_series_and_stores_the_bars() {
    // R-385
    let e = env("ingest");
    ok(&run(&e, &["calendar", "sync"]));
    let out = run(&e, &["data", "ingest"]);
    ok(&out);
    let text = stdout(&out).to_lowercase();
    for series in ["raw", "adjusted"] {
        let line = text
            .lines()
            .find(|l| l.contains(series))
            .unwrap_or_else(|| panic!("no {series} line in: {text}"));
        for word in ["inserted", "unchanged", "restated"] {
            assert!(line.contains(word), "{series} line lacks {word}: {line}");
        }
        // 3 symbols x 41 sessions inserted on the first ingest.
        assert!(
            line.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|t| t == "123"),
            "{series} line should show 123 inserted: {line}"
        );
    }
    let store = Store::open(&e.db).unwrap();
    for (adjustment, name) in [(Adjustment::Raw, "raw"), (Adjustment::All, "all")] {
        let bars = store
            .bars_asof(
                &Symbol::new("SPY").unwrap(),
                adjustment,
                "sip",
                d(1, 1),
                d(3, 31),
                None,
            )
            .unwrap();
        assert_eq!(bars.len(), 41, "{name} SPY bars");
    }
}

#[test]
fn data_check_exits_zero_after_sync_and_ingest() {
    // R-385
    let e = env("ingest");
    ok(&run(&e, &["calendar", "sync"]));
    ok(&run(&e, &["data", "ingest"]));
    let out = run(&e, &["data", "check"]);
    ok(&out);
}
