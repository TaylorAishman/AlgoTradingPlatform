//! R-137: the execution bar comes from the series named by the price basis.

use chrono::{NaiveDate, TimeZone, Utc};
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

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()
}

fn row(adjustment: Adjustment, close: i64) -> BarRow {
    let p = Price::new(Decimal::new(close, 0)).unwrap();
    BarRow {
        bar: Bar {
            symbol: spy(),
            date: day(),
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 1000,
        },
        adjustment,
        feed: "iex".to_string(),
    }
}

fn store() -> Store {
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap();
    store
        .upsert_bars(&[row(Adjustment::Raw, 100), row(Adjustment::All, 90)], t)
        .unwrap();
    store
}

fn load(store: &Store, basis: PriceBasis) -> StoreView {
    StoreView::load(store, &[spy()], "iex", basis, None).unwrap()
}

#[test]
fn raw_basis_view_returns_the_raw_close_from_exec_bar() {
    let store = store();
    let view = load(&store, PriceBasis::Raw);
    let bar = view.exec_bar(&spy(), day()).unwrap();
    assert_eq!(bar.close.value(), Decimal::new(100, 0));
}

#[test]
fn adjusted_basis_view_returns_the_adjusted_close_from_exec_bar() {
    let store = store();
    let view = load(&store, PriceBasis::Adjusted);
    let bar = view.exec_bar(&spy(), day()).unwrap();
    assert_eq!(bar.close.value(), Decimal::new(90, 0));
}

#[test]
fn adj_closes_is_adjusted_under_both_bases() {
    let store = store();
    for basis in [PriceBasis::Raw, PriceBasis::Adjusted] {
        let view = load(&store, basis);
        let got = view.adj_closes(&spy(), day(), 10);
        assert_eq!(got, vec![(day(), 90.0)]);
    }
}

#[test]
fn exec_bar_is_none_for_a_date_with_no_bar() {
    let store = store();
    let view = load(&store, PriceBasis::Raw);
    let other = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
    assert!(view.exec_bar(&spy(), other).is_none());
}
