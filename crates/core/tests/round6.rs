use pt_core::money::round6;
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// R-013
#[test]
fn positive_midpoint_rounds_away_from_zero() {
    assert_eq!(round6(d("1.0000005")), d("1.000001"));
}

// R-013
#[test]
fn negative_midpoint_rounds_away_from_zero() {
    assert_eq!(round6(d("-1.0000005")), d("-1.000001"));
}

// R-013
#[test]
fn below_midpoint_rounds_down() {
    assert_eq!(round6(d("2.0000004")), d("2.000000"));
}

// R-013
#[test]
fn even_digit_midpoint_still_rounds_up_not_to_even() {
    assert_eq!(round6(d("1.0000025")), d("1.000003"));
}

// R-013
#[test]
fn value_with_six_places_is_unchanged() {
    assert_eq!(round6(d("3.123456")), d("3.123456"));
}
