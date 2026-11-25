//! R-148: several lookbacks are blended with equal weight.

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
            d(2024, 3, 28),
            d(2024, 8, 30),
            d(2024, 10, 31),
            d(2024, 11, 29),
        ])
        .expect("ascending sessions"),
    )
}

fn view() -> MemoryView {
    let mut v = MemoryView::new();
    for date in [
        d(2024, 3, 28),
        d(2024, 8, 30),
        d(2024, 10, 31),
        d(2024, 11, 29),
    ] {
        v.push_adj_close(&sym("R"), date, 100.0);
    }
    for (date, close) in [
        (d(2024, 10, 31), 90.0),
        (d(2024, 8, 30), 110.0),
        (d(2024, 3, 28), 95.0),
        (d(2024, 11, 29), 100.0),
    ] {
        v.push_adj_close(&sym("A"), date, close);
    }
    v
}

fn tsmom(lookbacks: Vec<u32>) -> Tsmom {
    Tsmom::new(
        TsmomParams {
            lookback_months: lookbacks,
            risk_free_symbol: sym("R"),
        },
        calendar(),
    )
}

#[test]
fn signs_of_plus_one_minus_one_plus_one_blend_to_one_third() {
    // R-148
    let set = tsmom(vec![1, 3, 8])
        .signals(&[sym("A")], d(2024, 11, 29), &view())
        .expect("signals at a session");
    assert!((set.signals[&sym("A")] - 1.0 / 3.0).abs() < 1e-12);
    assert!(set.ineligible.is_empty());
}

#[test]
fn each_lookback_alone_gives_its_own_sign() {
    // R-148
    for (months, expected) in [(1, 1.0), (3, -1.0), (8, 1.0)] {
        let set = tsmom(vec![months])
            .signals(&[sym("A")], d(2024, 11, 29), &view())
            .expect("signals at a session");
        assert!((set.signals[&sym("A")] - expected).abs() < 1e-12);
    }
}

#[test]
fn lookback_order_does_not_change_the_blend() {
    // R-148
    let set = tsmom(vec![8, 1, 3])
        .signals(&[sym("A")], d(2024, 11, 29), &view())
        .expect("signals at a session");
    assert!((set.signals[&sym("A")] - 1.0 / 3.0).abs() < 1e-12);
}
