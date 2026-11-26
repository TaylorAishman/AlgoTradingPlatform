//! R-449: a momentum strategy with no lookback is an error, not a signal of 0.

use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::error::StrategyError;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use std::sync::Arc;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).expect("valid symbol")
}

#[test]
fn empty_lookback_months_is_an_invalid_error() {
    // R-449
    let cal = TradingCalendar::new(vec![d(2024, 3, 28), d(2024, 11, 29)]).expect("ascending");
    let strategy = Tsmom::new(
        TsmomParams {
            lookback_months: vec![],
            risk_free_symbol: sym("R"),
        },
        Arc::new(cal),
    );
    let mut v = MemoryView::new();
    for s in ["R", "A"] {
        for date in [d(2024, 3, 28), d(2024, 11, 29)] {
            v.push_adj_close(&sym(s), date, 100.0);
        }
    }
    let err = strategy
        .signals(&[sym("A")], d(2024, 11, 29), &v)
        .expect_err("no lookback must be an error");
    assert!(matches!(err, StrategyError::Invalid(_)), "got {err:?}");
}
