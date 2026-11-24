use pt_core::cost::CostSpec;
use pt_core::money::{Money, Price};
use pt_engine::cost::fill_cost;
use rust_decimal::Decimal;

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn price(n: i64) -> Price {
    Price::new(dec(n)).unwrap()
}

// R-190
#[test]
fn spread_of_ten_bps_on_500_shares_at_101_costs_50_point_5() {
    let spec = CostSpec {
        spread_bps: dec(10),
        ..CostSpec::zero()
    };
    let cost = fill_cost(&spec, 500, price(101), None).unwrap();
    assert_eq!(cost, Money(Decimal::new(505, 1)));
}

// R-190
#[test]
fn spread_and_fee_add_up_to_12_on_1000_shares_at_20() {
    let spec = CostSpec {
        spread_bps: dec(5),
        fee_bps: dec(1),
        ..CostSpec::zero()
    };
    let cost = fill_cost(&spec, 1000, price(20), None).unwrap();
    assert_eq!(cost, Money(dec(12)));
}

// R-190
#[test]
fn fee_alone_is_charged_on_notional() {
    let spec = CostSpec {
        fee_bps: dec(1),
        ..CostSpec::zero()
    };
    let cost = fill_cost(&spec, 1000, price(20), None).unwrap();
    assert_eq!(cost, Money(dec(2)));
}

// R-190
#[test]
fn zero_spec_costs_nothing() {
    let cost = fill_cost(&CostSpec::zero(), 500, price(101), None).unwrap();
    assert_eq!(cost, Money(Decimal::ZERO));
}

// R-190
#[test]
fn zero_shares_cost_nothing() {
    let spec = CostSpec {
        spread_bps: dec(10),
        fee_bps: dec(1),
        ..CostSpec::zero()
    };
    let cost = fill_cost(&spec, 0, price(101), None).unwrap();
    assert_eq!(cost, Money(Decimal::ZERO));
}

// R-190
#[test]
fn spread_and_fee_cost_is_rounded_to_six_places() {
    // 3 shares at 0.3333 = 0.9999 notional; 1 bp of that is 0.00009999, rounded to 0.0001.
    let spec = CostSpec {
        spread_bps: dec(1),
        ..CostSpec::zero()
    };
    let p = Price::new(Decimal::new(3333, 4)).unwrap();
    let cost = fill_cost(&spec, 3, p, None).unwrap();
    assert_eq!(cost, Money(Decimal::new(100, 6)));
}
