//! R-394 (report) and R-401 (logging): after the cycle `trader run` writes
//! `status.json` and `status.md`, whether the cycle ended ok or failed, and
//! every stderr line of the run is a JSON log line free of the secret.

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

/// The broker calendar: the bar sessions plus March 2024 through the 28th,
/// so 2024-02-29 is a month end and 2024-03-01 is the trade session. Bars
/// stay on the 41 sessions above, as in the live fixture.
fn calendar_sessions() -> Vec<NaiveDate> {
    let mut out = sessions();
    let mut day = d(3, 1);
    while day <= d(3, 28) {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
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

/// The live fixture: stage `paper`, the mock pre-open on 2024-03-01.
fn env() -> Env {
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
    std::fs::write(
        &owner,
        "stage = \"paper\"\nstrategy_allowlist = [\"tsmom-v1\"]\n\
         account_flat_confirmed_on = 2024-02-29\n",
    )
    .unwrap();

    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    let days = sessions();
    let dates: Vec<String> = calendar_sessions().iter().map(|s| s.to_string()).collect();
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
    let close_of = |name: &str, i: i64| -> f64 {
        match name {
            "SPY" => (100 + i + (i % 2)) as f64,
            "TLT" => (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0,
            _ => (10000 + i) as f64 / 100.0,
        }
    };
    let mut bars = Vec::new();
    for (i, date) in days.iter().enumerate() {
        let i = i as i64;
        for name in ["SPY", "TLT", "BIL"] {
            let close = close_of(name, i);
            let open = close_of(name, (i - 1).max(0));
            bars.push((
                name.to_string(),
                date.to_string(),
                open,
                open.max(close),
                open.min(close),
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

fn status_json(e: &Env) -> serde_json::Value {
    let path = e.dir.path().join("reports").join("status.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    serde_json::from_str(&text).expect("status.json is JSON")
}

#[test]
fn run_writes_status_json_with_stage_paper_and_status_md() {
    // R-394
    let e = env();
    let out = run(&e, &["run"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(status_json(&e)["stage"], "paper");
    assert!(e.dir.path().join("reports").join("status.md").exists());
}

#[test]
fn the_report_still_exists_when_the_broker_fails_every_request() {
    // R-394: exit 1 and the report is still written.
    let e = env();
    e.mock.fail_next(1000, 401);
    let out = run(&e, &["run"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert_eq!(status_json(&e)["stage"], "paper");
}

#[test]
fn every_stderr_line_of_a_run_is_json_and_hides_the_secret() {
    // R-401
    let e = env();
    let out = run(&e, &["run"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let err = stderr(&out);
    for line in err.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        for key in ["level", "message", "ts"] {
            assert!(
                v.get(key).and_then(|x| x.as_str()).is_some(),
                "missing string `{key}` in: {line}"
            );
        }
    }
    assert!(!err.contains("fake-secret"));
    assert!(!stdout(&out).contains("fake-secret"));
}

#[test]
fn an_active_halt_exits_two_and_the_report_lists_it() {
    // R-394: with an active halt it exits 2 and the report lists the halt.
    let e = env();
    {
        let store = pt_store::db::Store::open(&e.dir.path().join("test.sqlite")).unwrap();
        store
            .set_halt("manual", "holiday", chrono::Utc::now())
            .unwrap();
    }
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let status = status_json(&e);
    let halts = status["active_halts"]
        .as_array()
        .expect("active_halts array");
    assert_eq!(halts.len(), 1, "status.json: {status}");
    assert_eq!(halts[0]["reason"], "manual");
    assert_eq!(halts[0]["detail"], "holiday");
}
