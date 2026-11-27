//! Tests for the timestamp and book of the status report (R-524).

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use pt_store::trading::{Book, EquitySnapshot};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn config(dir: &std::path::Path) -> pt_config::config::Config {
    pt_config::config::parse(&format!(
        "[run]\noutput_dir = {:?}\n\n[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n",
        dir.to_str().unwrap()
    ))
    .unwrap()
}

fn write(store: &Store, cfg: &pt_config::config::Config, now: DateTime<Utc>) -> Value {
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity: &[],
        validation_signoff: None,
        now,
    };
    let (_md, json) = write_status(&inp).unwrap();
    serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap()
}

fn snap(store: &Store, date: NaiveDate, equity: &str) {
    store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date,
            cash: dec(equity),
            positions_value: dec("0"),
            equity: dec(equity),
            recorded_at: Utc.with_ymd_and_hms(2026, 3, 2, 22, 0, 0).unwrap(),
        })
        .unwrap();
}

// R-524: generated_at is the write time, equity is the latest stored snapshot's, even
// when that snapshot is for an earlier session than `now`.
#[test]
fn stamps_write_time_and_shows_latest_snapshot_equity() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    snap(&store, NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(), "100");
    snap(
        &store,
        NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(),
        "105.5",
    );
    let t = Utc.with_ymd_and_hms(2026, 3, 4, 15, 30, 0).unwrap();
    let v = write(&store, &cfg, t);
    assert_eq!(v["generated_at"], Value::String(t.to_rfc3339()));
    assert_eq!(v["equity"], Value::String("105.5".into()));
}

// R-524: a later `now` with no new data changes generated_at and no other key.
#[test]
fn later_write_with_no_new_data_changes_only_generated_at() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    snap(
        &store,
        NaiveDate::from_ymd_opt(2026, 2, 27).unwrap(),
        "105.5",
    );
    let t1 = Utc.with_ymd_and_hms(2026, 3, 4, 15, 30, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2026, 3, 5, 9, 0, 0).unwrap();
    let a = write(&store, &cfg, t1);
    let b = write(&store, &cfg, t2);
    assert_eq!(a["generated_at"], Value::String(t1.to_rfc3339()));
    assert_eq!(b["generated_at"], Value::String(t2.to_rfc3339()));
    let (mut a, mut b) = (a, b);
    a.as_object_mut().unwrap().remove("generated_at");
    b.as_object_mut().unwrap().remove("generated_at");
    assert_eq!(a, b);
}
