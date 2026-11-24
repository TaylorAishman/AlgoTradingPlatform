//! Input validation of the vector calculator (R-222).

use chrono::NaiveDate;
use pt_core::cost::CostSpec;
use pt_core::types::Symbol;
use pt_vector::error::VectorError;
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

fn base() -> VectorInputs {
    let mut weights = BTreeMap::new();
    weights.insert(d(3), vec![Decimal::new(5, 1), Decimal::new(-25, 2)]);
    VectorInputs {
        dates: vec![d(2), d(3), d(4)],
        symbols: vec![Symbol::new("A").unwrap(), Symbol::new("B").unwrap()],
        open: vec![row(100, 50), row(101, 51), row(102, 52)],
        close: vec![row(100, 50), row(101, 51), row(102, 52)],
        weights,
        days_to_next: vec![1, 1, 1],
        initial_capital: dec(100_000),
        cost: CostSpec::zero(),
    }
}

// R-222: a row of `open` with one column instead of two is a shape error.
#[test]
fn open_row_with_wrong_column_count_is_shape_error() {
    let mut inp = base();
    inp.open[1] = vec![dec(101)];
    assert!(matches!(vector_equity(&inp), Err(VectorError::Shape(_))));
}

// R-222: the same holds for a short `close` row.
#[test]
fn close_row_with_wrong_column_count_is_shape_error() {
    let mut inp = base();
    inp.close[2] = vec![dec(102)];
    assert!(matches!(vector_equity(&inp), Err(VectorError::Shape(_))));
}

// R-222: a weights row whose length differs from the symbol count is a shape error.
#[test]
fn weights_row_with_wrong_length_is_shape_error() {
    let mut inp = base();
    inp.weights.insert(d(3), vec![Decimal::new(5, 1)]);
    assert!(matches!(vector_equity(&inp), Err(VectorError::Shape(_))));
}

// R-222: a weights key not in `dates` returns UnknownTradeDate carrying that date.
#[test]
fn weights_key_not_in_dates_is_unknown_trade_date() {
    let mut inp = base();
    inp.weights.insert(d(9), vec![Decimal::ZERO, Decimal::ZERO]);
    assert_eq!(
        vector_equity(&inp),
        Err(VectorError::UnknownTradeDate(d(9)))
    );
}

// R-222: a price of 0 returns NonPositivePrice (open).
#[test]
fn zero_open_price_is_non_positive_price() {
    let mut inp = base();
    inp.open[1][0] = Decimal::ZERO;
    assert_eq!(vector_equity(&inp), Err(VectorError::NonPositivePrice));
}

// R-222: a price of 0 returns NonPositivePrice (close).
#[test]
fn zero_close_price_is_non_positive_price() {
    let mut inp = base();
    inp.close[0][1] = Decimal::ZERO;
    assert_eq!(vector_equity(&inp), Err(VectorError::NonPositivePrice));
}

// R-222: a negative price is also not positive.
#[test]
fn negative_price_is_non_positive_price() {
    let mut inp = base();
    inp.close[2][0] = dec(-1);
    assert_eq!(vector_equity(&inp), Err(VectorError::NonPositivePrice));
}

// R-222: impact_coeff = 1 returns ImpactNotSupported.
#[test]
fn impact_coefficient_is_not_supported() {
    let mut inp = base();
    inp.cost.impact_coeff = Decimal::ONE;
    assert_eq!(vector_equity(&inp), Err(VectorError::ImpactNotSupported));
}

// R-222: days_to_next of the wrong length returns LengthMismatch (too short).
#[test]
fn days_to_next_too_short_is_length_mismatch() {
    let mut inp = base();
    inp.days_to_next = vec![1, 1];
    assert_eq!(vector_equity(&inp), Err(VectorError::LengthMismatch));
}

// R-222: days_to_next of the wrong length returns LengthMismatch (too long).
#[test]
fn days_to_next_too_long_is_length_mismatch() {
    let mut inp = base();
    inp.days_to_next = vec![1, 1, 1, 1];
    assert_eq!(vector_equity(&inp), Err(VectorError::LengthMismatch));
}
