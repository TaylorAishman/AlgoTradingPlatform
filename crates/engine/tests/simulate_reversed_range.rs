mod common;

use chrono::NaiveDate;
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

fn config(start: NaiveDate, end: NaiveDate) -> SimConfig {
    SimConfig {
        start,
        end,
        initial_capital: dec(100000),
        cost: CostSpec::zero(),
        limits: RiskLimits {
            max_order_notional: dec(1000000),
            max_weight: dec(1),
            max_gross: dec(1),
            daily_loss_limit: dd("0.03"),
            max_drawdown: dd("0.15"),
        },
        universe: vec![sym("A")],
    }
}

fn calendar() -> TradingCalendar {
    TradingCalendar::new(vec![d(2024, 3, 1), d(2024, 3, 4), d(2024, 3, 5)]).unwrap()
}

// R-454
#[test]
fn start_after_end_is_invalid() {
    let cfg = config(d(2024, 3, 4), d(2024, 3, 1));
    let r = simulate(
        &cfg,
        &BTreeMap::<NaiveDate, Weights>::new(),
        &worked_view(),
        &calendar(),
    );
    assert!(matches!(r, Err(EngineError::Invalid(_))), "{r:?}");
}

// R-454
#[test]
fn start_equal_to_end_on_a_session_is_ok() {
    let cfg = config(d(2024, 3, 4), d(2024, 3, 4));
    let r = simulate(
        &cfg,
        &BTreeMap::<NaiveDate, Weights>::new(),
        &worked_view(),
        &calendar(),
    );
    let r = r.unwrap();
    assert_eq!(r.equity, vec![(d(2024, 3, 4), dec(100000))]);
}
