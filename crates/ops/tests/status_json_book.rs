//! Tests for the book keys of `status.json` (R-357).

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, LedgerEntry};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 2).unwrap()
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

/// Opening balance 100000, +10 SPY bought at 141, snapshot 100020 at SPY close 143.
fn traded_store(feed: &str) -> Store {
    let store = Store::open_in_memory().unwrap();
    let spy = Symbol::new("SPY").unwrap();
    store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: day(),
            kind: "opening".into(),
            symbol: None,
            qty: 0,
            price: None,
            cash_delta: dec("100000"),
            ref_id: None,
        })
        .unwrap();
    store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: day(),
            kind: "fill".into(),
            symbol: Some(spy.clone()),
            qty: 10,
            price: Some(dec("141")),
            cash_delta: dec("-1410"),
            ref_id: Some("f1".into()),
        })
        .unwrap();
    let close = Price::new(dec("143")).unwrap();
    store
        .upsert_bars(
            &[BarRow {
                bar: Bar {
                    symbol: spy,
                    date: day(),
                    open: close,
                    high: close,
                    low: close,
                    close,
                    volume: 1000,
                },
                adjustment: Adjustment::Raw,
                feed: feed.into(),
            }],
            now(),
        )
        .unwrap();
    store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date: day(),
            cash: dec("98590"),
            positions_value: dec("1430"),
            equity: dec("100020"),
            recorded_at: now(),
        })
        .unwrap();
    store
}

fn report(store: &Store, cfg: &pt_config::config::Config) -> Value {
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity: &[],
        validation_signoff: None,
        now: now(),
    };
    let (_md, json) = write_status(&inp).unwrap();
    serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap()
}

// R-357: status.json is created in cfg.run.output_dir with the fixed set of keys.
#[test]
fn status_json_is_written_to_output_dir_with_every_key() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let inp = ReportInputs {
        store: &store,
        cfg: &cfg,
        stage: "paper",
        fidelity: &[],
        validation_signoff: None,
        now: now(),
    };
    let (_md, json_path) = write_status(&inp).unwrap();
    assert_eq!(json_path, dir.path().join("status.json"));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
    let obj = v.as_object().unwrap();
    for key in [
        "generated_at",
        "stage",
        "feed",
        "basis",
        "last_run",
        "equity",
        "cash",
        "gross_exposure",
        "pnl",
        "positions",
        "last_rebalance",
        "active_halts",
        "trial_count",
        "fidelity",
        "validation_signoff",
        "disclaimer",
    ] {
        assert!(obj.contains_key(key), "missing key {key}");
    }
}

// R-357: stage, feed and basis come from the inputs and the config.
#[test]
fn stage_feed_and_basis_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let v = report(&store, &cfg);
    assert_eq!(v["stage"], "paper");
    assert_eq!(v["feed"], cfg.data.feed.as_str());
    assert_eq!(v["basis"], "price_return");
}

// R-357: equity and cash are decimal strings from the worked example.
#[test]
fn equity_and_cash_match_the_worked_example() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let v = report(&store, &cfg);
    assert_eq!(v["equity"], "100020");
    assert_eq!(v["cash"], "98590");
}

// R-357: one position, SPY, 10 shares, valued at 1430.
#[test]
fn positions_list_spy_with_quantity_and_value() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let v = report(&store, &cfg);
    let positions = v["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    let p = &positions[0];
    assert_eq!(p["symbol"], "SPY");
    assert_eq!(p["qty"].as_i64(), Some(10));
    assert_eq!(p["value"], "1430");
}

// R-357: gross exposure is 1430 / 100020 within 1e-9.
#[test]
fn gross_exposure_is_positions_over_equity() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let v = report(&store, &cfg);
    let g = v["gross_exposure"]
        .as_f64()
        .expect("gross_exposure is a number");
    assert!((g - 1430.0 / 100020.0).abs() < 1e-9, "got {g}");
}

// R-357: generated_at carries the time the report was made.
#[test]
fn generated_at_is_the_report_time() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let v = report(&store, &cfg);
    let got: DateTime<Utc> = v["generated_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(got, now());
}
