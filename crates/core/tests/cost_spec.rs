use pt_core::cost::CostSpec;
use pt_core::error::CoreError;
use rust_decimal::Decimal;

fn is_invalid_cost(r: Result<(), CoreError>) -> bool {
    matches!(r, Err(CoreError::InvalidCost(_)))
}

// R-016
#[test]
fn zero_has_all_four_fields_equal_to_zero() {
    let z = CostSpec::zero();
    assert_eq!(z.spread_bps, Decimal::ZERO);
    assert_eq!(z.fee_bps, Decimal::ZERO);
    assert_eq!(z.impact_coeff, Decimal::ZERO);
    assert_eq!(z.borrow_bps_annual, Decimal::ZERO);
}

// R-016
#[test]
fn zero_validates() {
    assert_eq!(CostSpec::zero().validate(), Ok(()));
}

// R-016
#[test]
fn negative_spread_is_rejected() {
    let mut c = CostSpec::zero();
    c.spread_bps = Decimal::from(-1);
    assert!(is_invalid_cost(c.validate()));
}

// R-016
#[test]
fn negative_fee_is_rejected() {
    let mut c = CostSpec::zero();
    c.fee_bps = Decimal::from(-1);
    assert!(is_invalid_cost(c.validate()));
}

// R-016
#[test]
fn negative_impact_coefficient_is_rejected() {
    let mut c = CostSpec::zero();
    c.impact_coeff = Decimal::from(-1);
    assert!(is_invalid_cost(c.validate()));
}

// R-016
#[test]
fn negative_borrow_is_rejected() {
    let mut c = CostSpec::zero();
    c.borrow_bps_annual = Decimal::from(-1);
    assert!(is_invalid_cost(c.validate()));
}

// R-016
#[test]
fn positive_parameters_validate() {
    let mut c = CostSpec::zero();
    c.spread_bps = Decimal::from(5);
    c.fee_bps = Decimal::from(1);
    c.impact_coeff = Decimal::new(1, 1);
    c.borrow_bps_annual = Decimal::from(30);
    assert_eq!(c.validate(), Ok(()));
}
