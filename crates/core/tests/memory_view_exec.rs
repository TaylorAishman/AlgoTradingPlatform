use chrono::NaiveDate;
use pt_core::money::Price;
use pt_core::types::{Bar, Symbol};
use pt_core::view::{MarketView, MemoryView};
use rust_decimal::Decimal;

fn d(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, month, day).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn bar(s: &str, date: NaiveDate, volume: u64) -> Bar {
    let p = Price::new(Decimal::new(1000, 2)).unwrap();
    Bar {
        symbol: sym(s),
        date,
        open: p,
        high: p,
        low: p,
        close: p,
        volume,
    }
}

fn view() -> MemoryView {
    let mut v = MemoryView::new();
    v.push_exec(bar("A", d(1, 2), 100));
    v.push_exec(bar("A", d(1, 3), 200));
    v.push_exec(bar("A", d(1, 4), 400));
    v
}

// R-018: exec_bar returns the bar pushed for the date.
#[test]
fn exec_bar_returns_pushed_bar() {
    assert_eq!(
        view().exec_bar(&sym("A"), d(1, 3)),
        Some(bar("A", d(1, 3), 200))
    );
}

// R-018: exec_bar is None for a date or symbol with no bar.
#[test]
fn exec_bar_none_when_absent() {
    let v = view();
    assert_eq!(v.exec_bar(&sym("A"), d(1, 5)), None);
    assert_eq!(v.exec_bar(&sym("B"), d(1, 3)), None);
}

// R-018: mean volume of the two latest bars on or before the date.
#[test]
fn adv_shares_mean_of_latest_bars() {
    assert_eq!(view().adv_shares(&sym("A"), d(1, 4), 2), Some(300.0));
}

// R-018: bars after the date are never used.
#[test]
fn adv_shares_ignores_later_bars() {
    assert_eq!(view().adv_shares(&sym("A"), d(1, 3), 2), Some(150.0));
}

// R-018: None if fewer than the requested sessions exist.
#[test]
fn adv_shares_none_when_too_few() {
    let v = view();
    assert_eq!(v.adv_shares(&sym("A"), d(1, 2), 2), None);
    assert_eq!(v.adv_shares(&sym("A"), d(1, 4), 4), None);
    assert_eq!(v.adv_shares(&sym("B"), d(1, 4), 1), None);
}
