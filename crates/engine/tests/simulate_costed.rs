mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::{Side, Weights};
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
        cost: CostSpec {
            spread_bps: dec(10),
            fee_bps: dec(0),
            impact_coeff: dec(0),
            borrow_bps_annual: dec(360),
        },
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

fn run() -> SimResult {
    simulate(&config(), &weights(), &worked_view(), &calendar()).unwrap()
}

// R-200
#[test]
fn costed_worked_example_equity_matches_to_the_last_digit() {
    assert_eq!(
        run().equity,
        vec![
            (d(2024, 1, 2), dec(100000)),
            (d(2024, 1, 3), dd("100922.60")),
            (d(2024, 1, 4), dd("101628.2602")),
        ]
    );
}

// R-200
#[test]
fn costed_worked_example_cost_total_is_170_point_291() {
    assert_eq!(run().cost_total, dd("170.291"));
}

// R-200
#[test]
fn costed_worked_example_borrow_total_is_4_point_4488() {
    assert_eq!(run().borrow_total, dd("4.4488"));
}

// R-200
#[test]
fn costed_worked_example_last_fill_is_a_sell_of_197_a() {
    let r = run();
    let (date, last) = r.fills.last().unwrap();
    assert_eq!(*date, d(2024, 1, 4));
    assert_eq!(last.symbol, sym("A"));
    assert_eq!(last.side, Side::Sell);
    assert_eq!(last.qty, 197);
}

// R-200
#[test]
fn costed_worked_example_has_no_rejections() {
    assert!(run().clean());
}
