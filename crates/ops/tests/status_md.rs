//! Tests for `status.md`, the report written for people (R-358).

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, LedgerEntry};
use rust_decimal::Decimal;
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

fn markdown(store: &Store, cfg: &pt_config::config::Config) -> (std::path::PathBuf, String) {
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity: &[],
        validation_signoff: None,
        now: now(),
    };
    let (md, _json) = write_status(&inp).unwrap();
    let text = std::fs::read_to_string(&md).expect("status.md was written");
    (md, text)
}

// R-358: status.md is created in the output directory.
#[test]
fn status_md_is_written_to_output_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let (md, _) = markdown(&store, &cfg);
    assert_eq!(md, dir.path().join("status.md"));
}

// R-358: status.md names the stage and the equity.
#[test]
fn status_md_contains_stage_and_equity() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let (_, text) = markdown(&store, &cfg);
    assert!(text.contains("paper"), "stage missing: {text}");
    assert!(text.contains("100020"), "equity missing: {text}");
}

// R-358: status.md has a positions table with the SPY row.
#[test]
fn status_md_has_a_positions_table_with_spy() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let (_, text) = markdown(&store, &cfg);
    let row = text
        .lines()
        .find(|l| l.contains("SPY"))
        .expect("a line for SPY");
    assert!(row.contains('|'), "SPY row is not a table row: {row}");
    assert!(row.contains("10"), "quantity missing: {row}");
    assert!(row.contains("1430"), "value missing: {row}");
}

// R-358: an active halt's reason appears in status.md.
#[test]
fn status_md_shows_active_halt_reason() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    store.set_halt("daily_loss", "down 3%", now()).unwrap();
    let (_, text) = markdown(&store, &cfg);
    assert!(text.contains("daily_loss"), "halt reason missing: {text}");
}

// R-358: a cleared halt's reason does not appear in status.md.
#[test]
fn status_md_omits_cleared_halt() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let id = store.set_halt("stale_data", "old", now()).unwrap();
    store.clear_halt(id, "ok", now()).unwrap();
    let (_, text) = markdown(&store, &cfg);
    assert!(!text.contains("stale_data"), "cleared halt shown: {text}");
}

// R-358: status.md says what it is: paper trading, and price returns only.
#[test]
fn status_md_carries_the_disclaimer_and_the_price_return_note() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = traded_store(&cfg.data.feed);
    let (_, text) = markdown(&store, &cfg);
    assert!(
        text.contains("Paper trading. Not investment advice."),
        "disclaimer missing: {text}"
    );
    assert!(
        text.contains("Returns are price returns: the paper account does not simulate dividends."),
        "price-return note missing: {text}"
    );
}
