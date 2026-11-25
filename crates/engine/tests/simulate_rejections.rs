mod common;

use chrono::NaiveDate;
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::{Side, Weights};
use pt_engine::simulate::{simulate, SimConfig};
use pt_risk::pretrade::{Rejection, RiskLimits};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn config(max_order_notional: i64) -> SimConfig {
    SimConfig {
        start: d(2024, 1, 2),
        end: d(2024, 1, 4),
        initial_capital: dec(100000),
        cost: CostSpec::zero(),
        limits: RiskLimits {
            max_order_notional: dec(max_order_notional),
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

// R-205
#[test]
fn rejected_buy_of_500_a_is_listed_with_its_date_and_reason() {
    let r = simulate(&config(40000), &weights(), &worked_view(), &calendar()).unwrap();
    let hit: Vec<_> = r
        .rejected
        .iter()
        .filter(|(date, o, _)| *date == d(2024, 1, 3) && o.symbol == sym("A"))
        .collect();
    assert_eq!(hit.len(), 1, "{:?}", r.rejected);
    let (_, o, reason) = hit[0];
    assert_eq!(o.side, Side::Buy);
    assert_eq!(o.qty, 500);
    assert_eq!(*reason, Rejection::OrderNotional);
}

// R-205
#[test]
fn rejected_buy_of_500_a_is_not_filled() {
    let r = simulate(&config(40000), &weights(), &worked_view(), &calendar()).unwrap();
    let bought = r
        .fills
        .iter()
        .any(|(date, f)| *date == d(2024, 1, 3) && f.symbol == sym("A") && f.side == Side::Buy);
    assert!(!bought, "{:?}", r.fills);
}

// R-205
#[test]
fn a_run_with_a_rejection_is_not_clean() {
    let r = simulate(&config(40000), &weights(), &worked_view(), &calendar()).unwrap();
    assert!(!r.rejected.is_empty());
    assert!(!r.clean());
}

// R-205
#[test]
fn the_approved_order_still_fills_when_another_is_rejected() {
    let r = simulate(&config(40000), &weights(), &worked_view(), &calendar()).unwrap();
    let sold_b = r.fills.iter().any(|(date, f)| {
        *date == d(2024, 1, 3) && f.symbol == sym("B") && f.side == Side::Sell && f.qty == 500
    });
    assert!(sold_b, "{:?}", r.fills);
}

// R-205 (R-199)
#[test]
fn the_worked_example_without_rejections_is_clean() {
    let r = simulate(&config(1000000), &weights(), &worked_view(), &calendar()).unwrap();
    assert!(r.rejected.is_empty());
    assert!(r.clean());
}
