//! R-136: the store-backed view serves adjusted closes as of an instant and never past the requested date.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, PriceBasis, Symbol};
use pt_core::view::HistoryView;
use pt_data::view::StoreView;
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;

fn spy() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, d).unwrap()
}

fn t1() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap()
}

fn t2() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 12, 0, 0).unwrap()
}

fn row(date: NaiveDate, close: i64) -> BarRow {
    let p = Price::new(Decimal::new(close, 0)).unwrap();
    BarRow {
        bar: Bar {
            symbol: spy(),
            date,
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 1000,
        },
        adjustment: Adjustment::All,
        feed: "iex".to_string(),
    }
}

/// Closes 100..=104 on Jan 2..=6 at t1, all restated at t2 to 90..=94.
fn store() -> Store {
    let store = Store::open_in_memory().unwrap();
    let original: Vec<BarRow> = (0..5).map(|i| row(day(2 + i), 100 + i as i64)).collect();
    let restated: Vec<BarRow> = (0..5).map(|i| row(day(2 + i), 90 + i as i64)).collect();
    store.upsert_bars(&original, t1()).unwrap();
    store.upsert_bars(&restated, t2()).unwrap();
    store
}

fn load(store: &Store, asof: Option<DateTime<Utc>>) -> StoreView {
    StoreView::load(store, &[spy()], "iex", PriceBasis::Adjusted, asof).unwrap()
}

fn values(v: &[(NaiveDate, f64)]) -> Vec<f64> {
    v.iter().map(|(_, c)| *c).collect()
}

#[test]
fn view_loaded_as_of_t1_returns_the_original_closes() {
    let store = store();
    let view = load(&store, Some(t1()));
    let got = view.adj_closes(&spy(), day(6), 10);
    assert_eq!(values(&got), vec![100.0, 101.0, 102.0, 103.0, 104.0]);
}

#[test]
fn view_loaded_without_instant_returns_the_restated_closes() {
    let store = store();
    let view = load(&store, None);
    let got = view.adj_closes(&spy(), day(6), 10);
    assert_eq!(values(&got), vec![90.0, 91.0, 92.0, 93.0, 94.0]);
}

#[test]
fn adj_closes_never_goes_past_the_requested_date_and_is_oldest_first() {
    let store = store();
    let view = load(&store, None);
    let got = view.adj_closes(&spy(), day(4), 10);
    assert_eq!(
        got.iter().map(|(d, _)| *d).collect::<Vec<_>>(),
        vec![day(2), day(3), day(4)]
    );
    assert_eq!(values(&got), vec![90.0, 91.0, 92.0]);
}

#[test]
fn adj_closes_returns_at_most_n_of_the_latest_dates() {
    let store = store();
    let view = load(&store, None);
    let got = view.adj_closes(&spy(), day(6), 2);
    assert_eq!(
        got.iter().map(|(d, _)| *d).collect::<Vec<_>>(),
        vec![day(5), day(6)]
    );
    assert_eq!(values(&got), vec![93.0, 94.0]);
}

#[test]
fn adj_closes_before_any_data_or_for_unknown_symbol_is_empty() {
    let store = store();
    let view = load(&store, None);
    assert!(view.adj_closes(&spy(), day(1), 10).is_empty());
    assert!(view
        .adj_closes(&Symbol::new("TLT").unwrap(), day(6), 10)
        .is_empty());
}
