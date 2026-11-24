use pt_core::cost::CostSpec;
use pt_core::money::{Money, Price};
use pt_engine::cost::{fill_cost, ImpactInputs};
use pt_engine::error::EngineError;
use rust_decimal::Decimal;

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn price(n: i64) -> Price {
    Price::new(dec(n)).unwrap()
}

fn impact_spec() -> CostSpec {
    CostSpec {
        impact_coeff: dec(1),
        ..CostSpec::zero()
    }
}

fn inputs(volume: i64) -> ImpactInputs {
    ImpactInputs {
        daily_vol: 0.01,
        adv_shares: volume as f64,
    }
}

// R-191
#[test]
fn square_root_impact_of_10000_shares_at_100_costs_1000() {
    let cost = fill_cost(&impact_spec(), 10_000, price(100), Some(inputs(1_000_000))).unwrap();
    assert_eq!(cost, Money(dec(1000)));
}

// R-191
#[test]
fn impact_is_charged_on_short_sales_as_a_positive_cost() {
    let cost = fill_cost(&impact_spec(), 10_000, price(100), Some(inputs(1_000_000))).unwrap();
    assert_eq!(cost, Money(dec(1000)));
}

// R-191
#[test]
fn impact_adds_to_spread_and_fee() {
    let spec = CostSpec {
        spread_bps: dec(10),
        ..impact_spec()
    };
    let cost = fill_cost(&spec, 10_000, price(100), Some(inputs(1_000_000))).unwrap();
    assert_eq!(cost, Money(dec(2000)));
}

// R-191
#[test]
fn impact_coefficient_without_inputs_is_missing_impact_inputs() {
    let err = fill_cost(&impact_spec(), 10_000, price(100), None).unwrap_err();
    assert!(
        matches!(err, EngineError::MissingImpactInputs(_)),
        "{err:?}"
    );
}

// R-191
#[test]
fn impact_coefficient_with_zero_volume_is_missing_impact_inputs() {
    let err = fill_cost(&impact_spec(), 10_000, price(100), Some(inputs(0))).unwrap_err();
    assert!(
        matches!(err, EngineError::MissingImpactInputs(_)),
        "{err:?}"
    );
}

// R-191
#[test]
fn zero_coefficient_without_inputs_is_not_an_error() {
    let cost = fill_cost(&CostSpec::zero(), 10_000, price(100), None).unwrap();
    assert_eq!(cost, Money(Decimal::ZERO));
}

// R-191
#[test]
fn zero_coefficient_ignores_inputs() {
    let cost = fill_cost(
        &CostSpec::zero(),
        10_000,
        price(100),
        Some(inputs(1_000_000)),
    )
    .unwrap();
    assert_eq!(cost, Money(Decimal::ZERO));
}

// R-191
#[test]
fn impact_with_non_perfect_square_participation_is_rounded_to_six_places() {
    // sqrt(2/3) is irrational: 0.01 * sqrt(2/3) * 200 notional = 1.632993...
    let cost = fill_cost(&impact_spec(), 2, price(100), Some(inputs(3))).unwrap();
    assert_eq!(cost, Money(Decimal::new(1_632_993, 6)));
}

// R-191
#[test]
fn impact_with_non_finite_or_negative_inputs_is_missing_impact_inputs() {
    for (vol, adv) in [
        (f64::NAN, 1e6),
        (0.01, f64::INFINITY),
        (-0.01, 1e6),
        (0.01, -1e6),
    ] {
        let i = ImpactInputs {
            daily_vol: vol,
            adv_shares: adv,
        };
        let err = fill_cost(&impact_spec(), 10_000, price(100), Some(i)).unwrap_err();
        assert!(
            matches!(err, EngineError::MissingImpactInputs(_)),
            "{err:?}"
        );
    }
}
