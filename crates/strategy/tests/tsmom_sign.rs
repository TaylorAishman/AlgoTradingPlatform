//! R-147: the signal is the sign of the excess return over the lookback.

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

fn calendar() -> Arc<TradingCalendar> {
    // 2024-03-29 is a holiday and is left out.
    Arc::new(
        TradingCalendar::new(vec![
            d(2024, 1, 31),
            d(2024, 2, 29),
            d(2024, 3, 28),
            d(2024, 4, 1),
        ])
        .expect("ascending sessions"),
    )
}

fn view() -> MemoryView {
    let mut v = MemoryView::new();
    for (s, a, b) in [
        ("R", 100.0, 102.0),
        ("A", 100.0, 110.0),
        ("B", 100.0, 101.0),
        ("C", 100.0, 102.0),
    ] {
        v.push_adj_close(&sym(s), d(2024, 1, 31), a);
        v.push_adj_close(&sym(s), d(2024, 2, 29), b);
    }
    v
}

fn tsmom() -> Tsmom {
    Tsmom::new(
        TsmomParams {
            lookback_months: vec![1],
            risk_free_symbol: sym("R"),
        },
        calendar(),
    )
}

#[test]
fn signal_is_the_sign_of_the_excess_return_over_the_risk_free() {
    // R-147
    let universe = [sym("A"), sym("B"), sym("C")];
    let set = tsmom()
        .signals(&universe, d(2024, 2, 29), &view())
        .expect("signals at a month end");
    assert_eq!(set.asof, d(2024, 2, 29));
    assert!((set.signals[&sym("A")] - 1.0).abs() < 1e-12);
    assert!((set.signals[&sym("B")] + 1.0).abs() < 1e-12);
    assert!(set.signals[&sym("C")].abs() < 1e-12);
    assert!(set.ineligible.is_empty());
}

#[test]
fn positive_return_below_the_risk_free_return_is_short() {
    // R-147
    let set = tsmom()
        .signals(&[sym("B")], d(2024, 2, 29), &view())
        .expect("signals at a month end");
    assert!(set.signals[&sym("B")] < 0.0);
}
