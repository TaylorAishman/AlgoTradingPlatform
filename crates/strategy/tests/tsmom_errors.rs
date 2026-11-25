//! R-150, R-151: a missing risk-free price stops the calculation, and signals
//! are only requested for a session.

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

fn view_with_rf(rf_dates: &[NaiveDate]) -> MemoryView {
    let mut v = MemoryView::new();
    for &date in rf_dates {
        v.push_adj_close(&sym("R"), date, 100.0);
    }
    for date in [d(2024, 3, 28), d(2024, 11, 29)] {
        v.push_adj_close(&sym("A"), date, 100.0);
    }
    v
}

#[test]
fn missing_risk_free_close_on_the_anchor_date_is_an_error_carrying_that_date() {
    // R-150
    let v = view_with_rf(&[d(2024, 11, 29)]);
    let err = tsmom()
        .signals(&[sym("A")], d(2024, 11, 29), &v)
        .expect_err("missing risk-free anchor must stop the calculation");
    assert_eq!(err, StrategyError::MissingRiskFree(d(2024, 3, 28)));
}

#[test]
fn missing_risk_free_close_on_the_asof_date_is_an_error_carrying_that_date() {
    // R-150
    let v = view_with_rf(&[d(2024, 3, 28)]);
    let err = tsmom()
        .signals(&[sym("A")], d(2024, 11, 29), &v)
        .expect_err("missing risk-free price must stop the calculation");
    assert_eq!(err, StrategyError::MissingRiskFree(d(2024, 11, 29)));
}

#[test]
fn asof_on_a_holiday_is_not_a_session() {
    // R-151
    let v = view_with_rf(&[d(2024, 3, 28), d(2024, 11, 29)]);
    let err = tsmom()
        .signals(&[sym("A")], d(2024, 3, 29), &v)
        .expect_err("a holiday is not a session");
    assert_eq!(err, StrategyError::NotASession(d(2024, 3, 29)));
}
