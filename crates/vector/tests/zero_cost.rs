//! Zero-cost accounting of the vector calculator (R-220).

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

fn worked_example() -> VectorInputs {
    let mut weights = BTreeMap::new();
    weights.insert(d(3), vec![Decimal::new(5, 1), Decimal::new(-25, 2)]);
    weights.insert(d(4), vec![Decimal::new(-2, 1), Decimal::ZERO]);
    VectorInputs {
        dates: vec![d(2), d(3), d(4)],
        symbols: vec![Symbol::new("A").unwrap(), Symbol::new("B").unwrap()],
        open: vec![row(100, 50), row(101, 49), row(103, 47)],
        close: vec![row(100, 50), row(102, 48), row(104, 46)],
        weights,
        days_to_next: vec![1, 1, 1],
        initial_capital: dec(100_000),
        cost: CostSpec::zero(),
    }
}

fn single(weight: Decimal) -> VectorInputs {
    let mut weights = BTreeMap::new();
    weights.insert(d(3), vec![weight]);
    VectorInputs {
        dates: vec![d(2), d(3)],
        symbols: vec![Symbol::new("A").unwrap()],
        open: vec![vec![Decimal::new(33333, 2)], vec![Decimal::new(33333, 2)]],
        close: vec![vec![Decimal::new(33333, 2)], vec![dec(340)]],
        weights,
        days_to_next: vec![1, 1],
        initial_capital: dec(100_000),
        cost: CostSpec::zero(),
    }
}

// R-220: the worked example at zero cost returns [100000, 101000, 101802].
#[test]
fn worked_example_at_zero_cost() {
    let eq = vector_equity(&worked_example()).unwrap();
    assert_eq!(eq, vec![dec(100_000), dec(101_000), dec(101_802)]);
}

// R-220: with no weights nothing trades and equity stays at the capital.
#[test]
fn no_weights_keeps_initial_capital() {
    let mut inp = worked_example();
    inp.weights.clear();
    let eq = vector_equity(&inp).unwrap();
    assert_eq!(eq, vec![dec(100_000); 3]);
}

// R-220: target shares truncate toward zero (0.1 * 100000 / 333.33 = 30.003 -> 30 long).
#[test]
fn long_target_truncates_toward_zero() {
    let eq = vector_equity(&single(Decimal::new(1, 1))).unwrap();
    // cash 100000 - 30 * 333.33 = 90000.10; marked 30 * 340 = 10200.
    assert_eq!(eq, vec![dec(100_000), Decimal::new(10_020_010, 2)]);
}

// R-220: the same truncation for a short (-30 shares).
#[test]
fn short_target_truncates_toward_zero() {
    let eq = vector_equity(&single(Decimal::new(-1, 1))).unwrap();
    // cash 100000 + 30 * 333.33 = 109999.90; marked -30 * 340 = -10200.
    assert_eq!(eq, vec![dec(100_000), Decimal::new(9_979_990, 2)]);
}
