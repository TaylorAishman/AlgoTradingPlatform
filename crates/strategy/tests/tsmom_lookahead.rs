//! R-146: signals do not change when data after `asof` changes.

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
    Arc::new(
        TradingCalendar::new(vec![
            d(2024, 2, 29),
            d(2024, 3, 28),
            d(2024, 4, 30),
            d(2024, 5, 31),
            d(2024, 6, 28),
        ])
        .expect("ascending sessions"),
    )
}

/// Closes through 2024-05-31 are the same in every view; `later` are the
/// closes on 2024-06-28, the only date after the decision date.
fn view(later: [f64; 3]) -> MemoryView {
    let mut v = MemoryView::new();
    let early = [
        ("R", [100.0, 100.5, 101.0, 101.5]),
        ("A", [100.0, 105.0, 110.0, 120.0]),
        ("B", [100.0, 95.0, 90.0, 80.0]),
    ];
    let dates = [
        d(2024, 2, 29),
        d(2024, 3, 28),
        d(2024, 4, 30),
        d(2024, 5, 31),
    ];
    for (i, (s, closes)) in early.iter().enumerate() {
        for (date, c) in dates.iter().zip(closes) {
            v.push_adj_close(&sym(s), *date, *c);
        }
        v.push_adj_close(&sym(s), d(2024, 6, 28), later[i]);
    }
    v
}

fn tsmom() -> Tsmom {
    Tsmom::new(
        TsmomParams {
            lookback_months: vec![1, 3],
            risk_free_symbol: sym("R"),
        },
        calendar(),
    )
}

#[test]
fn signals_are_equal_when_closes_after_asof_differ() {
    // R-146
    let universe = [sym("A"), sym("B")];
    let asof = d(2024, 5, 31);
    let one = tsmom()
        .signals(&universe, asof, &view([101.0, 150.0, 10.0]))
        .expect("signals");
    let two = tsmom()
        .signals(&universe, asof, &view([500.0, 1.0, 900.0]))
        .expect("signals");
    assert_eq!(one, two);
    assert!((one.signals[&sym("A")] - 1.0).abs() < 1e-12);
    assert!((one.signals[&sym("B")] + 1.0).abs() < 1e-12);
}

#[test]
fn signals_are_equal_when_data_after_asof_is_absent() {
    // R-146
    let universe = [sym("A"), sym("B")];
    let asof = d(2024, 5, 31);
    let with_later = tsmom()
        .signals(&universe, asof, &view([500.0, 1.0, 900.0]))
        .expect("signals");
    let mut truncated = MemoryView::new();
    for (s, closes) in [
        ("R", [100.0, 100.5, 101.0, 101.5]),
        ("A", [100.0, 105.0, 110.0, 120.0]),
        ("B", [100.0, 95.0, 90.0, 80.0]),
    ] {
        for (date, c) in [
            d(2024, 2, 29),
            d(2024, 3, 28),
            d(2024, 4, 30),
            d(2024, 5, 31),
        ]
        .iter()
        .zip(closes)
        {
            truncated.push_adj_close(&sym(s), *date, c);
        }
    }
    let without = tsmom()
        .signals(&universe, asof, &truncated)
        .expect("signals");
    assert_eq!(with_later, without);
}
