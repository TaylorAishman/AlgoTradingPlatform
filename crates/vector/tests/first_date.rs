//! A weights entry on the first date is not traded (R-461).

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

// Prices move between the first open and close, and costs are non-zero, so a
// trade on the first date would change the first equity value.
fn inputs(first_date_weight: Option<Decimal>) -> VectorInputs {
    let mut weights = BTreeMap::new();
    if let Some(w) = first_date_weight {
        weights.insert(d(2), vec![w]);
    }
    weights.insert(d(3), vec![Decimal::new(5, 1)]);
    VectorInputs {
        dates: vec![d(2), d(3), d(4)],
        symbols: vec![Symbol::new("A").unwrap()],
        open: vec![vec![dec(100)], vec![dec(101)], vec![dec(103)]],
        close: vec![vec![dec(110)], vec![dec(102)], vec![dec(104)]],
        weights,
        days_to_next: vec![1, 1, 1],
        initial_capital: dec(100_000),
        cost: CostSpec {
            spread_bps: dec(10),
            fee_bps: dec(5),
            borrow_bps_annual: dec(300),
            ..CostSpec::zero()
        },
    }
}

// R-461: a first-date weights entry changes nothing, and day one equals the capital.
#[test]
fn first_date_weights_entry_is_not_traded() {
    let without = vector_equity(&inputs(None)).unwrap();
    let long = vector_equity(&inputs(Some(Decimal::ONE))).unwrap();
    let short = vector_equity(&inputs(Some(-Decimal::ONE))).unwrap();
    assert_eq!(without[0], dec(100_000));
    assert_eq!(long, without);
    assert_eq!(short, without);
}
