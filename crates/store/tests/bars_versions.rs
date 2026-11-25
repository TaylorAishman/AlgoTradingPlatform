//! R-054: bars are stored as versions and never overwritten.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::market::{BarRow, UpsertStats};
use rusqlite::Connection;
use rust_decimal::Decimal;
use std::path::PathBuf;
use std::str::FromStr;

fn t(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, h, 0, 0).unwrap()
}

fn price(s: &str) -> Price {
    Price::new(Decimal::from_str(s).unwrap()).unwrap()
}

fn row(symbol: &str, day: u32, close: &str) -> BarRow {
    BarRow {
        bar: Bar {
            symbol: Symbol::new(symbol).unwrap(),
            date: NaiveDate::from_ymd_opt(2024, 1, day).unwrap(),
            open: price("99.5"),
            high: price("101.25"),
            low: price("98.75"),
            close: price(close),
            volume: 1_000,
        },
        adjustment: Adjustment::Raw,
        feed: "sip".to_string(),
    }
}

fn stats(inserted: usize, unchanged: usize, restated: usize) -> UpsertStats {
    UpsertStats {
        inserted,
        unchanged,
        restated,
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    store: Store,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let store = Store::open(&path).unwrap();
    Fixture {
        _dir: dir,
        path,
        store,
    }
}

fn count(f: &Fixture) -> i64 {
    Connection::open(&f.path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM bars", [], |r| r.get(0))
        .unwrap()
}

/// Closes of the stored versions of SPY 2024-01-02, oldest fetch first.
fn closes(f: &Fixture) -> Vec<String> {
    let conn = Connection::open(&f.path).unwrap();
    let mut stmt = conn
        .prepare("SELECT close FROM bars WHERE symbol = 'SPY' AND date = '2024-01-02' ORDER BY fetched_at")
        .unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn first_bar_is_inserted() {
    let f = fixture();
    let s = f.store.upsert_bars(&[row("SPY", 2, "100")], t(1)).unwrap();
    assert_eq!(s, stats(1, 0, 0));
    assert_eq!(count(&f), 1);
}

#[test]
fn identical_bar_at_later_time_is_unchanged_and_adds_no_row() {
    let f = fixture();
    f.store.upsert_bars(&[row("SPY", 2, "100")], t(1)).unwrap();
    let s = f.store.upsert_bars(&[row("SPY", 2, "100")], t(2)).unwrap();
    assert_eq!(s, stats(0, 1, 0));
    assert_eq!(count(&f), 1);
}

#[test]
fn changed_close_is_restated_as_a_new_version_keeping_the_old() {
    let f = fixture();
    f.store.upsert_bars(&[row("SPY", 2, "100")], t(1)).unwrap();
    f.store.upsert_bars(&[row("SPY", 2, "100")], t(2)).unwrap();
    let s = f
        .store
        .upsert_bars(&[row("SPY", 2, "100.5")], t(3))
        .unwrap();
    assert_eq!(s, stats(0, 0, 1));
    assert_eq!(count(&f), 2);
    let got: Vec<Decimal> = closes(&f)
        .iter()
        .map(|c| Decimal::from_str(c).unwrap())
        .collect();
    assert_eq!(
        got,
        vec![
            Decimal::from_str("100").unwrap(),
            Decimal::from_str("100.5").unwrap()
        ]
    );
}

#[test]
fn unchanged_is_judged_against_the_latest_version() {
    let f = fixture();
    f.store.upsert_bars(&[row("SPY", 2, "100")], t(1)).unwrap();
    f.store.upsert_bars(&[row("SPY", 2, "101")], t(2)).unwrap();
    // Back to the first value: differs from the latest version, so restated.
    let s = f.store.upsert_bars(&[row("SPY", 2, "100")], t(3)).unwrap();
    assert_eq!(s, stats(0, 0, 1));
    assert_eq!(count(&f), 3);
    // Repeating the latest value is unchanged.
    let s = f.store.upsert_bars(&[row("SPY", 2, "100")], t(4)).unwrap();
    assert_eq!(s, stats(0, 1, 0));
    assert_eq!(count(&f), 3);
}

#[test]
fn a_change_in_any_field_counts_as_a_restatement() {
    let f = fixture();
    f.store.upsert_bars(&[row("SPY", 2, "100")], t(1)).unwrap();
    let mut changed = row("SPY", 2, "100");
    changed.bar.volume = 1_001;
    let s = f.store.upsert_bars(&[changed], t(2)).unwrap();
    assert_eq!(s, stats(0, 0, 1));
    assert_eq!(count(&f), 2);
}

#[test]
fn a_batch_counts_each_row_by_its_own_outcome() {
    let f = fixture();
    f.store
        .upsert_bars(&[row("SPY", 2, "100"), row("SPY", 3, "101")], t(1))
        .unwrap();
    let s = f
        .store
        .upsert_bars(
            &[
                row("SPY", 2, "100"), // unchanged
                row("SPY", 3, "102"), // restated
                row("SPY", 4, "103"), // new
                row("QQQ", 2, "400"), // new symbol
            ],
            t(2),
        )
        .unwrap();
    assert_eq!(s, stats(2, 1, 1));
    assert_eq!(count(&f), 5);
}

#[test]
fn an_empty_batch_changes_nothing() {
    let f = fixture();
    let s = f.store.upsert_bars(&[], t(1)).unwrap();
    assert_eq!(s, stats(0, 0, 0));
    assert_eq!(count(&f), 0);
}

#[test]
fn other_adjustment_or_feed_is_a_separate_series_not_a_restatement() {
    let f = fixture();
    f.store.upsert_bars(&[row("SPY", 2, "100")], t(1)).unwrap();
    let mut adjusted = row("SPY", 2, "90");
    adjusted.adjustment = Adjustment::All;
    let mut iex = row("SPY", 2, "101");
    iex.feed = "iex".to_string();
    let s = f.store.upsert_bars(&[adjusted, iex], t(2)).unwrap();
    assert_eq!(s, stats(2, 0, 0));
    assert_eq!(count(&f), 3);
}

#[test]
fn prices_keep_full_decimal_precision() {
    let f = fixture();
    let p = "123.456789012345678";
    f.store.upsert_bars(&[row("SPY", 2, p)], t(1)).unwrap();
    let got = closes(&f);
    assert_eq!(got.len(), 1);
    assert_eq!(
        Decimal::from_str(&got[0]).unwrap(),
        Decimal::from_str(p).unwrap()
    );
}
