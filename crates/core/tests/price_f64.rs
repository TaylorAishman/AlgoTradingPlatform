use pt_core::error::CoreError;
use pt_core::money::Price;
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// R-012
#[test]
fn float_becomes_the_decimal_it_prints_as() {
    let p = Price::from_f64_shortest(123.456789).unwrap();
    assert_eq!(p.value(), d("123.456789"));
}

// R-012
#[test]
fn point_one_is_exactly_one_tenth() {
    let p = Price::from_f64_shortest(0.1).unwrap();
    assert_eq!(p.value(), d("0.1"));
}

// R-012
#[test]
fn whole_number_float_keeps_its_value() {
    let p = Price::from_f64_shortest(101.0).unwrap();
    assert_eq!(p.value(), d("101"));
}

// R-012
#[test]
fn nan_is_not_finite() {
    assert_eq!(
        Price::from_f64_shortest(f64::NAN),
        Err(CoreError::NotFinite)
    );
}

// R-012
#[test]
fn infinities_are_not_finite() {
    assert_eq!(
        Price::from_f64_shortest(f64::INFINITY),
        Err(CoreError::NotFinite)
    );
    assert_eq!(
        Price::from_f64_shortest(f64::NEG_INFINITY),
        Err(CoreError::NotFinite)
    );
}

// R-012
#[test]
fn zero_is_a_non_positive_price() {
    assert_eq!(
        Price::from_f64_shortest(0.0),
        Err(CoreError::NonPositivePrice)
    );
}

// R-012
#[test]
fn negative_float_is_a_non_positive_price() {
    assert_eq!(
        Price::from_f64_shortest(-1.5),
        Err(CoreError::NonPositivePrice)
    );
}
