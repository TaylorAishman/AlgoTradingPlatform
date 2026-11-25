mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::{Side, Weights};
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

// R-199
#[test]
fn zero_cost_worked_example_equity_matches_to_the_last_digit() {
    let r = simulate(&config(), &weights(), &worked_view(), &calendar()).unwrap();
    assert_eq!(
        r.equity,
        vec![
            (d(2024, 1, 2), dec(100000)),
            (d(2024, 1, 3), dec(101000)),
            (d(2024, 1, 4), dec(101802)),
        ]
    );
}

// R-199
#[test]
fn zero_cost_worked_example_totals() {
    let r = simulate(&config(), &weights(), &worked_view(), &calendar()).unwrap();
    assert_eq!(r.traded_notional, dec(170394));
    assert_eq!(r.cost_total, dec(0));
    assert_eq!(r.borrow_total, dec(0));
    assert!(r.rejected.is_empty());
    assert!(r.clean());
}

// R-199
#[test]
fn zero_cost_worked_example_fills_on_the_second_trade_date_in_order() {
    let r = simulate(&config(), &weights(), &worked_view(), &calendar()).unwrap();
    let on_04: Vec<_> = r
        .fills
        .iter()
        .filter(|(date, _)| *date == d(2024, 1, 4))
        .map(|(_, f)| (f.symbol.clone(), f.side, f.qty, f.price))
        .collect();
    assert_eq!(
        on_04,
        vec![
            (sym("A"), Side::Sell, 500, price(103)),
            (sym("B"), Side::Buy, 500, price(47)),
            (sym("A"), Side::Sell, 198, price(103)),
        ]
    );
    let on_03: Vec<_> = r
        .fills
        .iter()
        .filter(|(date, _)| *date == d(2024, 1, 3))
        .map(|(_, f)| (f.symbol.clone(), f.side, f.qty, f.price))
        .collect();
    assert_eq!(
        on_03,
        vec![
            (sym("B"), Side::Sell, 500, price(49)),
            (sym("A"), Side::Buy, 500, price(101)),
        ]
    );
    assert_eq!(r.fills.len(), 5);
}
