//! R-149: a symbol without a price at a lookback anchor is ineligible, not an error.

use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use std::sync::Arc;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).expect("valid symbol")
}

fn tsmom() -> Tsmom {
    let cal = TradingCalendar::new(vec![d(2024, 3, 28), d(2024, 11, 29)]).expect("ascending");
    Tsmom::new(
        TsmomParams {
            lookback_months: vec![8],
            risk_free_symbol: sym("R"),
        },
        Arc::new(cal),
    )
}

fn view() -> MemoryView {
    let mut v = MemoryView::new();
    for date in [d(2024, 3, 28), d(2024, 11, 29)] {
        v.push_adj_close(&sym("R"), date, 100.0);
        v.push_adj_close(
            &sym("OK"),
            date,
            if date.month0() == 2 { 90.0 } else { 100.0 },
        );
    }
    // NEW has a close now but none on the anchor 2024-03-28.
    v.push_adj_close(&sym("NEW"), d(2024, 11, 29), 50.0);
    v
}

use chrono::Datelike;

#[test]
fn symbol_without_anchor_close_is_ineligible_with_insufficient_history() {
    // R-149
    let set = tsmom()
        .signals(&[sym("NEW"), sym("OK")], d(2024, 11, 29), &view())
        .expect("a missing anchor price is not an error");
    assert!(!set.signals.contains_key(&sym("NEW")));
    assert_eq!(set.ineligible[&sym("NEW")], "insufficient_history");
}

#[test]
fn other_symbols_are_unaffected_by_an_ineligible_one() {
    // R-149
    let set = tsmom()
        .signals(&[sym("NEW"), sym("OK")], d(2024, 11, 29), &view())
        .expect("signals");
    assert!((set.signals[&sym("OK")] - 1.0).abs() < 1e-12);
    assert!(!set.ineligible.contains_key(&sym("OK")));
    assert_eq!(set.signals.len(), 1);
    assert_eq!(set.ineligible.len(), 1);
}
