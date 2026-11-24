use pt_core::cost::CostSpec;
use pt_core::money::Money;
use pt_engine::cost::borrow_cost;
use rust_decimal::Decimal;

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn spec(annual_bps: i64) -> CostSpec {
    CostSpec {
        borrow_bps_annual: dec(annual_bps),
        ..CostSpec::zero()
    }
}

// R-192
#[test]
fn one_day_of_borrow_at_360_bps_on_24000_is_2_point_4() {
    let cost = borrow_cost(&spec(360), Money(dec(24000)), 1);
    assert_eq!(cost, Money(Decimal::new(24, 1)));
}

// R-192
#[test]
fn three_days_of_borrow_accrue_by_calendar_days() {
    let cost = borrow_cost(&spec(360), Money(dec(24000)), 3);
    assert_eq!(cost, Money(Decimal::new(72, 1)));
}

// R-192
#[test]
fn borrow_on_zero_short_value_is_zero() {
    let cost = borrow_cost(&spec(360), Money(dec(0)), 1);
    assert_eq!(cost, Money(dec(0)));
}

// R-192
#[test]
fn zero_borrow_rate_costs_nothing() {
    let cost = borrow_cost(&CostSpec::zero(), Money(dec(24000)), 3);
    assert_eq!(cost, Money(dec(0)));
}

// R-192
#[test]
fn borrow_is_rounded_to_six_places() {
    // 1000 * 1 / 10000 / 360 = 0.000277777..., rounded to 0.000278.
    let cost = borrow_cost(&spec(1), Money(dec(1000)), 1);
    assert_eq!(cost, Money(Decimal::new(278, 6)));
}
