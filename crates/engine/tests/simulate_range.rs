mod common;

use chrono::NaiveDate;
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::Weights;
use pt_engine::error::EngineError;
use pt_engine::simulate::{simulate, SimConfig, SimResult};
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

fn weights() -> BTreeMap<NaiveDate, Weights> {
    let mut w1 = Weights::new();
    w1.insert(sym("A"), dd("0.5"));
    w1.insert(sym("B"), dd("-0.25"));
    BTreeMap::from([(d(2024, 1, 3), w1)])
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

fn is_invalid(r: &Result<SimResult, EngineError>) -> bool {
    matches!(r, Err(EngineError::Invalid(_)))
}

// R-203
#[test]
fn equity_has_one_point_per_session_with_matching_dates() {
    let cal = calendar();
    let r = simulate(&config(), &weights(), &worked_view(), &cal).unwrap();
    let dates: Vec<NaiveDate> = r.equity.iter().map(|(date, _)| *date).collect();
    let sessions: Vec<NaiveDate> = cal.sessions_between(d(2024, 1, 2), d(2024, 1, 4)).to_vec();
    assert_eq!(sessions.len(), 3);
    assert_eq!(dates, sessions);
}

// R-203
#[test]
fn end_beyond_the_last_session_is_invalid() {
    let mut cfg = config();
    cfg.end = d(2024, 1, 8);
    let r = simulate(&cfg, &weights(), &worked_view(), &calendar());
    assert!(is_invalid(&r), "{r:?}");
}

// R-203
#[test]
fn start_that_is_not_a_session_is_invalid() {
    let mut cfg = config();
    cfg.start = d(2024, 1, 1);
    let r = simulate(&cfg, &weights(), &worked_view(), &calendar());
    assert!(is_invalid(&r), "{r:?}");
}

// R-204
#[test]
fn weights_keyed_on_a_non_session_date_are_invalid() {
    let mut w = weights();
    w.insert(d(2024, 1, 6), Weights::new());
    let r = simulate(&config(), &w, &worked_view(), &calendar());
    assert!(is_invalid(&r), "{r:?}");
}

// R-204
#[test]
fn weights_keyed_on_a_session_after_the_range_are_invalid() {
    let mut w = weights();
    w.insert(d(2024, 1, 5), Weights::new());
    let r = simulate(&config(), &w, &worked_view(), &calendar());
    assert!(is_invalid(&r), "{r:?}");
}

// R-204
#[test]
fn weights_keyed_on_a_session_before_the_range_are_invalid() {
    let mut cfg = config();
    cfg.start = d(2024, 1, 3);
    let mut w = weights();
    w.insert(d(2024, 1, 2), Weights::new());
    let r = simulate(&cfg, &w, &worked_view(), &calendar());
    assert!(is_invalid(&r), "{r:?}");
}
