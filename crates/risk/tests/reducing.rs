use std::collections::BTreeMap;

use pt_core::money::{Money, Price};
use pt_core::types::{OrderIntent, Positions, Side, Symbol};
use pt_risk::pretrade::{check_orders, GateOutcome, Rejection, RiskLimits};
use rust_decimal::Decimal;

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn px(n: i64) -> Price {
    Price::new(Decimal::new(n, 0)).unwrap()
}

fn limits(max_order_notional: i64) -> RiskLimits {
    RiskLimits {
        max_order_notional: Decimal::new(max_order_notional, 0),
        max_weight: Decimal::new(1, 0),
        max_gross: Decimal::new(1, 0),
        daily_loss_limit: Decimal::new(3, 2),
        max_drawdown: Decimal::new(15, 2),
    }
}

fn order(side: Side, qty: u64) -> OrderIntent {
    OrderIntent::new(sym("A"), side, qty, 1, px(100)).unwrap()
}

fn run(held: i64, orders: &[OrderIntent], max_order_notional: i64) -> GateOutcome {
    let mut positions = Positions::new();
    positions.insert(sym("A"), held);
    let prices = BTreeMap::from([(sym("A"), px(100))]);
    check_orders(
        orders,
        &positions,
        Money(Decimal::new(100_000, 0)),
        &prices,
        &limits(max_order_notional),
    )
}

// R-183
#[test]
fn sell_reducing_an_overweight_long_is_approved() {
    let out = run(1500, &[order(Side::Sell, 200)], 1_000_000);
    assert_eq!(out.approved.len(), 1);
    assert!(out.rejected.is_empty());
}

// R-183
#[test]
fn buy_reducing_an_overweight_short_is_approved() {
    let out = run(-1500, &[order(Side::Buy, 200)], 1_000_000);
    assert_eq!(out.approved.len(), 1);
    assert!(out.rejected.is_empty());
}

// R-183
#[test]
fn reducing_order_is_still_subject_to_the_order_notional_limit() {
    let out = run(1500, &[order(Side::Sell, 200)], 10_000);
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].1, Rejection::OrderNotional);
}

// R-183
#[test]
fn order_that_flips_to_an_overweight_opposite_position_is_not_reducing() {
    // Long 1500 -> sell 3200 leaves short 1700 (170%), larger than before in magnitude.
    let out = run(1500, &[order(Side::Sell, 3200)], 1_000_000);
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected[0].1, Rejection::WeightCap);
}

// R-183
#[test]
fn order_increasing_an_overweight_long_is_still_rejected() {
    let out = run(1500, &[order(Side::Buy, 100)], 1_000_000);
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected[0].1, Rejection::WeightCap);
}

// R-183: the gross exemption is per order; a reducing order survives a batch
// that is rejected for gross, while the order that pushes gross over is not.
#[test]
fn reducing_order_is_approved_while_a_gross_breaching_order_in_the_batch_is_rejected() {
    let mut positions = Positions::new();
    positions.insert(sym("A"), 1500);
    let prices = BTreeMap::from([(sym("A"), px(100)), (sym("B"), px(50))]);
    let orders = [
        order(Side::Sell, 200),
        OrderIntent::new(sym("B"), Side::Buy, 10, 1, px(50)).unwrap(),
    ];
    let out = check_orders(
        &orders,
        &positions,
        Money(Decimal::new(100_000, 0)),
        &prices,
        &limits(1_000_000),
    );
    assert_eq!(out.approved.len(), 1);
    assert_eq!(out.approved[0].symbol, sym("A"));
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].0.symbol, sym("B"));
    assert_eq!(out.rejected[0].1, Rejection::Gross);
}
