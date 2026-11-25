//! Tests for the run, P&L, halt, trial, fidelity and sign-off keys of `status.json` (R-357).

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use pt_store::ops::TrialRow;
use pt_store::trading::{Book, EquitySnapshot};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, 22, 0, 0).unwrap()
}

fn config(dir: &std::path::Path) -> pt_config::config::Config {
    pt_config::config::parse(&format!(
        "[run]\noutput_dir = {:?}\n\n[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n",
        dir.to_str().unwrap()
    ))
    .unwrap()
}

fn snap(store: &Store, date: NaiveDate, equity: &str) {
    store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date,
            cash: dec(equity),
            positions_value: dec("0"),
            equity: dec(equity),
            recorded_at: now(),
        })
        .unwrap();
}

fn report_with(
    store: &Store,
    cfg: &pt_config::config::Config,
    fidelity: &[(String, String)],
    signoff: Option<&str>,
) -> Value {
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity,
        validation_signoff: signoff,
        now: now(),
    };
    let (_md, json) = write_status(&inp).unwrap();
    serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap()
}

/// A number or a decimal string, as f64.
fn num(v: &Value) -> f64 {
    match v {
        Value::String(s) => s.parse().unwrap(),
        Value::Number(n) => n.as_f64().unwrap(),
        other => panic!("not a number: {other}"),
    }
}

// R-357: last_run describes the latest run and its status.
#[test]
fn last_run_reports_the_latest_run_status() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let t = now() - Duration::hours(3);
    let a = store.start_run("paper", "h", "v", t).unwrap();
    store.finish_run(a, "ok", "", t).unwrap();
    let b = store.start_run("paper", "h", "v", now()).unwrap();
    store.finish_run(b, "failed", "", now()).unwrap();
    let v = report_with(&store, &cfg, &[], None);
    let run = v["last_run"].as_object().expect("last_run is an object");
    assert_eq!(run["status"], "failed");
}

// R-357: pnl holds day, mtd, ytd and inception from the snapshot series.
#[test]
fn pnl_reports_day_and_inception_returns() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    snap(
        &store,
        NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(),
        "100000",
    );
    snap(
        &store,
        NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
        "100020",
    );
    let v = report_with(&store, &cfg, &[], None);
    let p = v["pnl"].as_object().expect("pnl is an object");
    for k in ["day", "mtd", "ytd", "inception"] {
        assert!((num(&p[k]) - 0.0002).abs() < 1e-9, "{k}: {}", p[k]);
    }
}

// R-357: pnl is null when there is no snapshot for the latest session date.
#[test]
fn pnl_is_null_without_a_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let v = report_with(&store, &cfg, &[], None);
    assert!(v["pnl"].is_null());
}

// R-357: active_halts lists the reasons of halts not yet cleared.
#[test]
fn active_halts_lists_uncleared_reasons_only() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    store.set_halt("daily_loss", "down 3%", now()).unwrap();
    let cleared = store.set_halt("stale_data", "old", now()).unwrap();
    store.clear_halt(cleared, "ok", now()).unwrap();
    let v = report_with(&store, &cfg, &[], None);
    let halts = v["active_halts"].as_array().unwrap();
    assert_eq!(halts.len(), 1);
    assert!(halts[0].to_string().contains("daily_loss"));
    assert!(!v["active_halts"].to_string().contains("stale_data"));
}

// R-357: trial_count is the number of registered trials of the strategy family.
#[test]
fn trial_count_counts_trials_of_the_strategy_family() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    for (family, hash) in [
        (cfg.strategy.id.as_str(), "a"),
        (cfg.strategy.id.as_str(), "b"),
        ("other-family", "c"),
    ] {
        store
            .register_trial(&TrialRow {
                family: family.into(),
                trial_hash: hash.into(),
                sharpe: 0.1,
                n_obs: 10,
                runs: 1,
            })
            .unwrap();
    }
    let v = report_with(&store, &cfg, &[], None);
    assert_eq!(v["trial_count"].as_u64(), Some(2));
}

// R-357: fidelity carries the label and value pairs given.
#[test]
fn fidelity_pairs_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let fid = vec![("sessions".to_string(), "63".to_string())];
    let v = report_with(&store, &cfg, &fid, None);
    let s = v["fidelity"].to_string();
    assert!(s.contains("sessions") && s.contains("63"), "got {s}");
}

// R-357: validation_signoff carries the owner file's value, or null.
#[test]
fn validation_signoff_is_reported_or_null() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let signed = report_with(&store, &cfg, &[], Some("signed 2026-03-01"));
    assert_eq!(signed["validation_signoff"], "signed 2026-03-01");
    let none = report_with(&store, &cfg, &[], None);
    assert!(none["validation_signoff"].is_null());
}

// R-357: the disclaimer key carries the not-advice statement.
#[test]
fn disclaimer_says_it_is_not_investment_advice() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let v = report_with(&store, &cfg, &[], None);
    let d = v["disclaimer"].as_str().expect("disclaimer is a string");
    assert!(d.contains("Not investment advice"), "got {d}");
}

// R-357: last_rebalance reflects the latest paper decision, and is null with none.
#[test]
fn last_rebalance_reflects_the_latest_decision() {
    use pt_store::trading::{DecisionRow, Mode};
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let v = report_with(&store, &cfg, &[], None);
    assert!(v["last_rebalance"].is_null());
    let mk = |sd: (i32, u32, u32), td: (i32, u32, u32), status: &str, mode: Mode| DecisionRow {
        id: 0,
        strategy_id: "s".to_string(),
        signal_date: NaiveDate::from_ymd_opt(sd.0, sd.1, sd.2).unwrap(),
        trade_date: Some(NaiveDate::from_ymd_opt(td.0, td.1, td.2).unwrap()),
        mode,
        status: status.to_string(),
        inputs_json: "{}".to_string(),
        outputs_json: "{}".to_string(),
        created_at: now(),
    };
    store
        .insert_decision(&mk((2025, 1, 31), (2025, 2, 3), "done", Mode::Paper))
        .unwrap();
    store
        .insert_decision(&mk((2025, 2, 28), (2025, 3, 3), "submitted", Mode::Paper))
        .unwrap();
    store
        .insert_decision(&mk((2025, 3, 31), (2025, 4, 1), "dry", Mode::DryRun))
        .unwrap();
    let v = report_with(&store, &cfg, &[], None);
    let r = v["last_rebalance"].as_object().expect("an object");
    assert_eq!(r["signal_date"], "2025-02-28");
    assert_eq!(r["trade_date"], "2025-03-03");
    assert_eq!(r["status"], "submitted");
}
