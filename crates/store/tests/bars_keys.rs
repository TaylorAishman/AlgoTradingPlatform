//! R-056: raw and adjusted bars and different feeds must not mix.

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

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn spy() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn row(close: &str, adjustment: Adjustment, feed: &str) -> BarRow {
    let p = |s: &str| Price::new(dec(s)).unwrap();
    BarRow {
        bar: Bar {
            symbol: spy(),
            date: day(),
            open: p("89"),
            high: p("102"),
            low: p("88"),
            close: p(close),
            volume: 1_000,
        },
        adjustment,
        feed: feed.to_string(),
    }
}

fn closes(s: &Store, adjustment: Adjustment, feed: &str) -> Vec<Decimal> {
    s.bars_asof(&spy(), adjustment, feed, day(), day(), None)
        .unwrap()
        .iter()
        .map(|b| b.close.value())
        .collect()
}

fn seeded() -> Store {
    let s = Store::open_in_memory().unwrap();
    let stats = s
        .upsert_bars(
            &[
                row("100", Adjustment::Raw, "sip"),
                row("90", Adjustment::All, "sip"),
                row("101", Adjustment::Raw, "iex"),
            ],
            t(1),
        )
        .unwrap();
    assert_eq!(stats.inserted, 3);
    assert_eq!(stats.restated, 0);
    s
}

#[test]
fn raw_sip_returns_only_its_own_bar() {
    // R-056
    assert_eq!(closes(&seeded(), Adjustment::Raw, "sip"), vec![dec("100")]);
}

#[test]
fn adjusted_sip_returns_only_its_own_bar() {
    // R-056
    assert_eq!(closes(&seeded(), Adjustment::All, "sip"), vec![dec("90")]);
}

#[test]
fn raw_iex_returns_only_its_own_bar() {
    // R-056
    assert_eq!(closes(&seeded(), Adjustment::Raw, "iex"), vec![dec("101")]);
}

#[test]
fn a_series_with_nothing_stored_is_empty() {
    // R-056
    assert!(closes(&seeded(), Adjustment::All, "iex").is_empty());
}

#[test]
fn restating_one_series_leaves_the_others_alone() {
    // R-056
    let s = seeded();
    let stats = s
        .upsert_bars(&[row("95", Adjustment::All, "sip")], t(2))
        .unwrap();
    assert_eq!(stats.restated, 1);
    assert_eq!(closes(&s, Adjustment::All, "sip"), vec![dec("95")]);
    assert_eq!(closes(&s, Adjustment::Raw, "sip"), vec![dec("100")]);
    assert_eq!(closes(&s, Adjustment::Raw, "iex"), vec![dec("101")]);
}
