//! R-138: a view reads one feed only, and average volume is the mean of the latest bars.

use chrono::{Duration, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, PriceBasis, Symbol};
use pt_core::view::{HistoryView, MarketView};
use pt_data::view::StoreView;
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;

fn spy() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn day(n: i64) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + Duration::days(n)
}

fn row(n: i64, adjustment: Adjustment, feed: &str, volume: u64) -> BarRow {
    let p = Price::new(Decimal::new(100, 0)).unwrap();
    BarRow {
        bar: Bar {
            symbol: spy(),
            date: day(n),
            open: p,
            high: p,
            low: p,
            close: p,
            volume,
        },
        adjustment,
        feed: feed.to_string(),
    }
}

/// `count` days (offsets 0..count) under `feed`, volume 1000 + 10 * offset, both series.
fn store_with(feed: &str, count: i64) -> Store {
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
    let mut rows = Vec::new();
    for n in 0..count {
        let v = 1000 + 10 * n as u64;
        rows.push(row(n, Adjustment::Raw, feed, v));
        rows.push(row(n, Adjustment::All, feed, v));
    }
    store.upsert_bars(&rows, t).unwrap();
    store
}

fn load(store: &Store, feed: &str) -> StoreView {
    StoreView::load(store, &[spy()], feed, PriceBasis::Raw, None).unwrap()
}

#[test]
fn view_for_another_feed_has_no_exec_bar_and_no_adjusted_closes() {
    // R-138
    let store = store_with("iex", 3);
    let view = load(&store, "sip");
    assert_eq!(view.exec_bar(&spy(), day(0)), None);
    assert!(view.adj_closes(&spy(), day(2), 10).is_empty());
}

#[test]
fn view_for_the_stored_feed_sees_the_bars() {
    // R-138
    let store = store_with("iex", 3);
    let view = load(&store, "iex");
    assert!(view.exec_bar(&spy(), day(0)).is_some());
    assert_eq!(view.adj_closes(&spy(), day(2), 10).len(), 3);
}

#[test]
fn adv_is_none_for_a_view_of_another_feed() {
    // R-138
    let store = store_with("iex", 25);
    let view = load(&store, "sip");
    assert_eq!(view.adv_shares(&spy(), day(24), 20), None);
}

#[test]
fn adv_is_the_mean_volume_of_the_latest_sessions_on_or_before_the_date() {
    // R-138: offsets 5..=24 have volumes 1050..=1240, mean 1145.
    let store = store_with("iex", 25);
    let view = load(&store, "iex");
    assert_eq!(view.adv_shares(&spy(), day(24), 20), Some(1145.0));
    // Asking on an earlier date uses offsets 0..=19, mean 1095.
    assert_eq!(view.adv_shares(&spy(), day(19), 20), Some(1095.0));
}

#[test]
fn adv_is_none_with_fewer_sessions_than_requested() {
    // R-138
    let store = store_with("iex", 19);
    let view = load(&store, "iex");
    assert_eq!(view.adv_shares(&spy(), day(18), 20), None);
    // Exactly 20 is enough.
    let store = store_with("iex", 20);
    let view = load(&store, "iex");
    assert!(view.adv_shares(&spy(), day(19), 20).is_some());
}

#[test]
fn adv_counts_only_sessions_on_or_before_the_date() {
    // R-138: 25 bars stored, but only 10 on or before day 9.
    let store = store_with("iex", 25);
    let view = load(&store, "iex");
    assert_eq!(view.adv_shares(&spy(), day(9), 20), None);
}
