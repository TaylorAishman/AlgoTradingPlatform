//! R-054, R-055: bars are readable as latest, or as they were at a past instant.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;
use std::str::FromStr;

fn t(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, h, 0, 0).unwrap()
}

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn spy() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn row(day: u32, close: &str) -> BarRow {
    let p = |s: &str| Price::new(dec(s)).unwrap();
    BarRow {
        bar: Bar {
            symbol: spy(),
            date: d(day),
            open: p("99.5"),
            high: p("101.25"),
            low: p("98.75"),
            close: p(close),
            volume: 1_000,
        },
        adjustment: Adjustment::Raw,
        feed: "sip".to_string(),
    }
}

fn store() -> Store {
    Store::open_in_memory().unwrap()
}

fn closes(s: &Store, from: u32, to: u32, asof: Option<DateTime<Utc>>) -> Vec<Decimal> {
    s.bars_asof(&spy(), Adjustment::Raw, "sip", d(from), d(to), asof)
        .unwrap()
        .iter()
        .map(|b| b.close.value())
        .collect()
}

/// The R-054 / R-055 scenario: t1 insert, t2 identical, t3 changed close.
fn scenario() -> Store {
    let s = store();
    s.upsert_bars(&[row(2, "100")], t(1)).unwrap();
    s.upsert_bars(&[row(2, "100")], t(2)).unwrap();
    s.upsert_bars(&[row(2, "100.5")], t(3)).unwrap();
    s
}

#[test]
fn none_returns_the_latest_version() {
    // R-054
    let s = scenario();
    assert_eq!(closes(&s, 2, 2, None), vec![dec("100.5")]);
}

#[test]
fn some_t2_returns_the_original_close() {
    // R-055
    let s = scenario();
    assert_eq!(closes(&s, 2, 2, Some(t(2))), vec![dec("100")]);
}

#[test]
fn instant_is_inclusive_of_a_version_fetched_exactly_then() {
    // R-055
    let s = scenario();
    assert_eq!(closes(&s, 2, 2, Some(t(1))), vec![dec("100")]);
    assert_eq!(closes(&s, 2, 2, Some(t(3))), vec![dec("100.5")]);
}

#[test]
fn instant_before_the_first_fetch_returns_no_bar() {
    // R-055
    let s = scenario();
    assert!(closes(&s, 2, 2, Some(t(0))).is_empty());
}

#[test]
fn instant_after_the_last_fetch_returns_the_latest_version() {
    // R-055
    let s = scenario();
    assert_eq!(closes(&s, 2, 2, Some(t(20))), vec![dec("100.5")]);
}

#[test]
fn each_date_is_judged_by_its_own_versions_at_the_instant() {
    // R-055
    let s = store();
    s.upsert_bars(&[row(2, "100")], t(1)).unwrap();
    s.upsert_bars(&[row(2, "110"), row(3, "103")], t(5))
        .unwrap();
    assert_eq!(closes(&s, 2, 3, Some(t(2))), vec![dec("100")]);
    assert_eq!(closes(&s, 2, 3, Some(t(5))), vec![dec("110"), dec("103")]);
    assert_eq!(closes(&s, 2, 3, None), vec![dec("110"), dec("103")]);
}

#[test]
fn results_are_ordered_by_date_ascending() {
    // R-055
    let s = store();
    s.upsert_bars(&[row(5, "105"), row(2, "102"), row(4, "104")], t(1))
        .unwrap();
    let dates: Vec<NaiveDate> = s
        .bars_asof(&spy(), Adjustment::Raw, "sip", d(1), d(31), None)
        .unwrap()
        .iter()
        .map(|b| b.date)
        .collect();
    assert_eq!(dates, vec![d(2), d(4), d(5)]);
}

#[test]
fn range_is_limited_to_from_and_to_inclusive() {
    // R-055
    let s = store();
    let rows: Vec<BarRow> = (2..=8)
        .map(|day| row(day, &format!("{}", 100 + day)))
        .collect();
    s.upsert_bars(&rows, t(1)).unwrap();
    assert_eq!(
        closes(&s, 3, 5, None),
        vec![dec("103"), dec("104"), dec("105")]
    );
    assert_eq!(closes(&s, 4, 4, None), vec![dec("104")]);
    assert!(closes(&s, 20, 25, None).is_empty());
}

#[test]
fn other_symbols_are_not_returned() {
    // R-055
    let s = store();
    let mut q = row(2, "400");
    q.bar.symbol = Symbol::new("QQQ").unwrap();
    s.upsert_bars(&[row(2, "100"), q], t(1)).unwrap();
    assert_eq!(closes(&s, 2, 2, None), vec![dec("100")]);
}

#[test]
fn an_empty_store_returns_no_bars() {
    // R-055
    let s = store();
    assert!(closes(&s, 1, 31, None).is_empty());
    assert!(closes(&s, 1, 31, Some(t(1))).is_empty());
}

#[test]
fn prices_keep_full_decimal_precision_when_read_back() {
    // R-053 via R-055
    let s = store();
    s.upsert_bars(&[row(2, "123.456789012345678")], t(1))
        .unwrap();
    assert_eq!(closes(&s, 2, 2, None), vec![dec("123.456789012345678")]);
}
