//! R-153: the benchmark is long every instrument that has a price.

use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::benchmark::LongOnly;
use pt_strategy::strategy::Strategy;
use std::sync::Arc;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).expect("valid symbol")
}

fn bench() -> LongOnly {
    let cal = TradingCalendar::new(vec![d(2024, 3, 27), d(2024, 3, 28)]).expect("ascending");
    LongOnly::new(Arc::new(cal))
}

fn view() -> MemoryView {
    let mut v = MemoryView::new();
    v.push_adj_close(&sym("A"), d(2024, 3, 28), 10.0);
    v.push_adj_close(&sym("B"), d(2024, 3, 28), 20.0);
    // OLD has a close only on the previous session, none on the decision date.
    v.push_adj_close(&sym("OLD"), d(2024, 3, 27), 5.0);
    v
}

#[test]
fn id_is_bh_volscaled_v1() {
    // R-153
    assert_eq!(bench().id(), "bh-volscaled-v1");
}

#[test]
fn every_symbol_with_a_close_on_the_date_gets_plus_one() {
    // R-153
    let set = bench()
        .signals(&[sym("A"), sym("B")], d(2024, 3, 28), &view())
        .expect("signals");
    assert_eq!(set.asof, d(2024, 3, 28));
    assert_eq!(set.signals.len(), 2);
    assert_eq!(set.signals[&sym("A")], 1.0);
    assert_eq!(set.signals[&sym("B")], 1.0);
    assert!(set.ineligible.is_empty());
}

#[test]
fn symbol_with_no_data_is_ineligible_with_no_price() {
    // R-153
    let set = bench()
        .signals(&[sym("A"), sym("NONE")], d(2024, 3, 28), &view())
        .expect("signals");
    assert_eq!(set.signals.len(), 1);
    assert_eq!(set.signals[&sym("A")], 1.0);
    assert_eq!(set.ineligible[&sym("NONE")], "no_price");
    assert_eq!(set.ineligible.len(), 1);
}

#[test]
fn symbol_priced_only_before_the_date_is_ineligible_with_no_price() {
    // R-153
    let set = bench()
        .signals(&[sym("OLD"), sym("B")], d(2024, 3, 28), &view())
        .expect("signals");
    assert!(!set.signals.contains_key(&sym("OLD")));
    assert_eq!(set.ineligible[&sym("OLD")], "no_price");
    assert_eq!(set.signals[&sym("B")], 1.0);
}
