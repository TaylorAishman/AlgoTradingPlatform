mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::Weights;
use pt_engine::error::EngineError;
use pt_engine::simulate::{simulate, SimConfig};
use pt_risk::pretrade::RiskLimits;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn config() -> SimConfig {
    SimConfig {
        start: d(2024, 1, 2),
        end: d(2024, 1, 4),
        initial_capital: dec(100000),
        cost: CostSpec::zero(),
        limits: RiskLimits {
            max_order_notional: dec(1000000),
            max_weight: dec(1),
            max_gross: dec(1),
            daily_loss_limit: dd("0.03"),
            max_drawdown: dd("0.15"),
        },
        universe: vec![sym("A"), sym("B")],
    }
}

fn weights() -> BTreeMap<chrono::NaiveDate, Weights> {
    let mut w1 = Weights::new();
    w1.insert(sym("A"), dd("0.5"));
    w1.insert(sym("B"), dd("-0.25"));
    let mut w2 = Weights::new();
    w2.insert(sym("A"), dd("-0.2"));
    w2.insert(sym("B"), dd("0"));
    BTreeMap::from([(d(2024, 1, 3), w1), (d(2024, 1, 4), w2)])
}

fn calendar() -> TradingCalendar {
    TradingCalendar::new(vec![
        d(2024, 1, 2),
        d(2024, 1, 3),
        d(2024, 1, 4),
        d(2024, 1, 5),
    ])
    .unwrap()
}

fn missing(symbol: &str, day: u32) -> EngineError {
    EngineError::MissingBar {
        symbol: symbol.to_string(),
        date: d(2024, 1, day),
    }
}

// R-202
#[test]
fn missing_bar_of_a_held_symbol_stops_the_run() {
    let view = worked_view_without(&[("A", 4)]);
    let r = simulate(&config(), &weights(), &view, &calendar());
    assert_eq!(r, Err(missing("A", 4)));
}

// R-202
#[test]
fn missing_bar_of_a_short_symbol_stops_the_run() {
    let view = worked_view_without(&[("B", 3)]);
    let r = simulate(&config(), &weights(), &view, &calendar());
    assert_eq!(r, Err(missing("B", 3)));
}

// R-202
#[test]
fn missing_bar_on_the_first_session_stops_the_run() {
    // The reference close for the first trade date is missing.
    let view = worked_view_without(&[("A", 2)]);
    let r = simulate(&config(), &weights(), &view, &calendar());
    assert_eq!(r, Err(missing("A", 2)));
}

// R-202
#[test]
fn complete_bars_do_not_error() {
    let r = simulate(&config(), &weights(), &worked_view(), &calendar());
    assert!(r.is_ok());
}
