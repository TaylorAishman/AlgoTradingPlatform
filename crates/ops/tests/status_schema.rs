//! Tests for the exact keys of `status.json` (R-523).

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use pt_store::market::BarRow;
use pt_store::trading::{Book, DecisionRow, EquitySnapshot, LedgerEntry, Mode};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::BTreeSet;
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

fn keys(v: &Value) -> BTreeSet<String> {
    v.as_object().expect("an object").keys().cloned().collect()
}

fn expect(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn write(store: &Store, cfg: &pt_config::config::Config) -> Value {
    let fid = vec![("label".to_string(), "value".to_string())];
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity: &fid,
        validation_signoff: Some("signed"),
        now: now(),
    };
    let (_md, json) = write_status(&inp).unwrap();
    serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap()
}

// R-523: top-level and nested keys are exactly the fixed set.
#[test]
fn status_json_has_exactly_the_fixed_keys() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let r = store.start_run("paper", "h", "v", now()).unwrap();
    store.finish_run(r, "ok", "", now()).unwrap();
    store.set_halt("daily_loss", "down 3%", now()).unwrap();
    store
        .insert_decision(&DecisionRow {
            id: 0,
            strategy_id: "s".into(),
            signal_date: day(),
            trade_date: Some(NaiveDate::from_ymd_opt(2026, 3, 3).unwrap()),
            mode: Mode::Paper,
            status: "planned".into(),
            inputs_json: "{}".into(),
            outputs_json: "{}".into(),
            created_at: now(),
        })
        .unwrap();
    let v = write(&store, &cfg);
    assert_eq!(
        keys(&v),
        expect(&[
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
        ])
    );
    assert_eq!(
        keys(&v["last_run"]),
        expect(&["id", "stage", "status", "started_at", "finished_at"])
    );
    assert_eq!(keys(&v["pnl"]), expect(&["day", "mtd", "ytd", "inception"]));
    let positions = v["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(keys(&positions[0]), expect(&["symbol", "qty", "value"]));
    assert_eq!(
        keys(&v["last_rebalance"]),
        expect(&["signal_date", "trade_date", "status", "created_at"])
    );
    let halts = v["active_halts"].as_array().unwrap();
    assert_eq!(halts.len(), 1);
    assert_eq!(keys(&halts[0]), expect(&["reason", "detail", "set_at"]));
    let fid = v["fidelity"].as_array().unwrap();
    assert_eq!(fid.len(), 1);
    assert_eq!(keys(&fid[0]), expect(&["label", "value"]));
}

// R-523: an absent P&L value is written as null, and the key stays.
#[test]
fn absent_pnl_value_is_written_as_null_with_all_four_keys() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    for (date, equity) in [
        (NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(), "100"),
        (NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(), "0"),
        (NaiveDate::from_ymd_opt(2026, 3, 3).unwrap(), "50"),
    ] {
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
    let v = write(&store, &cfg);
    assert_eq!(keys(&v["pnl"]), expect(&["day", "mtd", "ytd", "inception"]));
    assert!(v["pnl"]["day"].is_null());
}
