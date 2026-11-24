use pt_core::error::CoreError;
use pt_core::money::{Money, Price};
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// R-011
#[test]
fn zero_price_is_rejected() {
    assert_eq!(Price::new(d("0")).unwrap_err(), CoreError::NonPositivePrice);
}

// R-011
#[test]
fn negative_price_is_rejected() {
    assert_eq!(
        Price::new(d("-1")).unwrap_err(),
        CoreError::NonPositivePrice
    );
}

// R-011
#[test]
fn smallest_tested_positive_price_is_accepted_and_value_is_exact() {
    let p = Price::new(d("0.0001")).unwrap();
    assert_eq!(p.value(), d("0.0001"));
}

// R-014
#[test]
fn notional_of_short_position_is_exact_and_negative() {
    let p = Price::new(d("101.25")).unwrap();
    assert_eq!(p.notional(-3), Money(d("-303.75")));
}

// R-014
#[test]
fn notional_of_long_and_zero_shares() {
    let p = Price::new(d("101.25")).unwrap();
    assert_eq!(p.notional(4), Money(d("405.00")));
    assert_eq!(p.notional(0), Money(d("0")));
}

// R-014
#[test]
fn notional_has_no_binary_float_error() {
    let p = Price::new(d("0.1")).unwrap();
    assert_eq!(p.notional(3), Money(d("0.3")));
}
