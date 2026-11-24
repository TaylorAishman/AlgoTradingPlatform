//! Proportional cost and borrow in the vector calculator (R-221).

use chrono::NaiveDate;
use pt_core::cost::CostSpec;
use pt_core::types::Symbol;
use pt_vector::vector::{vector_equity, VectorInputs};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn row(a: i64, b: i64) -> Vec<Decimal> {
    vec![dec(a), dec(b)]
}

fn costed_spec() -> CostSpec {
    CostSpec {
        spread_bps: dec(10),
        fee_bps: Decimal::ZERO,
        impact_coeff: Decimal::ZERO,
        borrow_bps_annual: dec(360),
    }
}

fn costed_example(days_to_next: Vec<i64>, cost: CostSpec) -> VectorInputs {
    let mut weights = BTreeMap::new();
    weights.insert(d(3), vec![Decimal::new(5, 1), Decimal::new(-25, 2)]);
    weights.insert(d(4), vec![Decimal::new(-2, 1), Decimal::ZERO]);
    VectorInputs {
        dates: vec![d(2), d(3), d(4)],
        symbols: vec![Symbol::new("A").unwrap(), Symbol::new("B").unwrap()],
        open: vec![row(100, 50), row(101, 49), row(103, 47)],
        close: vec![row(100, 50), row(102, 48), row(104, 46)],
        weights,
        days_to_next,
        initial_capital: dec(100_000),
        cost,
    }
}

// R-221: the worked example with costs returns [100000, 100922.60, 101628.2602].
#[test]
fn worked_example_with_costs() {
    let eq = vector_equity(&costed_example(vec![1, 1, 1], costed_spec())).unwrap();
    assert_eq!(
        eq,
        vec![
            dec(100_000),
            Decimal::new(10_092_260, 2),
            Decimal::new(1_016_282_602, 4)
        ]
    );
}

// R-221: with days_to_next = [1, 3, 1] borrow covers the weekend, giving
// [100000, 100917.80, 101623.4602].
#[test]
fn borrow_accrues_over_calendar_days_to_next_session() {
    let eq = vector_equity(&costed_example(vec![1, 3, 1], costed_spec())).unwrap();
    assert_eq!(
        eq,
        vec![
            dec(100_000),
            Decimal::new(10_091_780, 2),
            Decimal::new(1_016_234_602, 4)
        ]
    );
}

// R-221: the spread cost alone (no borrow) is 75 on the first trade date.
#[test]
fn spread_cost_alone_is_charged_on_traded_notional() {
    let spec = CostSpec {
        borrow_bps_annual: Decimal::ZERO,
        ..costed_spec()
    };
    let eq = vector_equity(&costed_example(vec![1, 1, 1], spec)).unwrap();
    // Zero-cost equity on 01-03 is 101000; the spread on 50500 + 24500 is 75.
    assert_eq!(eq[1], dec(100_925));
}

// R-221: borrow alone (no spread) on 01-03 is 24000 * 360 / 10000 / 360 = 2.4, so 100997.6.
#[test]
fn borrow_alone_is_charged_on_short_market_value() {
    let spec = CostSpec {
        spread_bps: Decimal::ZERO,
        ..costed_spec()
    };
    let eq = vector_equity(&costed_example(vec![1, 1, 1], spec)).unwrap();
    assert_eq!(eq[1], Decimal::new(1_009_976, 1));
}
