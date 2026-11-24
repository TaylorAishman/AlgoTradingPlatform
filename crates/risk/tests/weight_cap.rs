use std::collections::BTreeMap;

use pt_core::money::{Money, Price};
use pt_core::types::{OrderIntent, Positions, Side, Symbol};
use pt_risk::pretrade::{check_orders, Rejection, RiskLimits};
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
        max_weight: Decimal::new(1, 1),
        max_gross: Decimal::new(100, 0),
        daily_loss_limit: Decimal::new(3, 2),
        max_drawdown: Decimal::new(15, 2),
    }
}

fn order(side: Side, qty: u64) -> OrderIntent {
    OrderIntent::new(sym("A"), side, qty, 1, px(100)).unwrap()
}

fn prices() -> BTreeMap<Symbol, Price> {
    BTreeMap::from([(sym("A"), px(100))])
}

fn equity(n: i64) -> Money {
    Money(Decimal::new(n, 0))
}

fn run(
    orders: &[OrderIntent],
    held: i64,
    eq: i64,
    max_notional: i64,
) -> pt_risk::pretrade::GateOutcome {
    let mut positions = Positions::new();
    if held != 0 {
        positions.insert(sym("A"), held);
    }
    check_orders(
        orders,
        &positions,
        equity(eq),
        &prices(),
        &limits(max_notional),
    )
}

// R-181
#[test]
fn buy_reaching_exactly_the_weight_cap_is_approved() {
    let out = run(&[order(Side::Buy, 100)], 0, 100000, 1_000_000);
    assert_eq!(out.approved.len(), 1);
    assert!(out.rejected.is_empty());
}

// R-181
#[test]
fn buy_above_the_weight_cap_is_rejected() {
    let out = run(&[order(Side::Buy, 101)], 0, 100000, 1_000_000);
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 1);
    assert!(matches!(out.rejected[0].1, Rejection::WeightCap));
}

// R-181
#[test]
fn short_sell_above_the_weight_cap_is_rejected() {
    let out = run(&[order(Side::Sell, 101)], 0, 100000, 1_000_000);
    assert!(matches!(out.rejected[0].1, Rejection::WeightCap));
    let ok = run(&[order(Side::Sell, 100)], 0, 100000, 1_000_000);
    assert_eq!(ok.approved.len(), 1);
}

// R-181
#[test]
fn existing_position_counts_towards_the_weight_cap() {
    let out = run(&[order(Side::Buy, 50)], 60, 100000, 1_000_000);
    assert!(matches!(out.rejected[0].1, Rejection::WeightCap));
    let ok = run(&[order(Side::Buy, 40)], 60, 100000, 1_000_000);
    assert_eq!(ok.approved.len(), 1);
}

// R-181 (each order sees the position left by approved orders before it)
#[test]
fn later_orders_see_the_position_left_by_approved_earlier_orders() {
    let out = run(
        &[order(Side::Buy, 60), order(Side::Buy, 50)],
        0,
        100000,
        1_000_000,
    );
    assert_eq!(out.approved.len(), 1);
    assert_eq!(out.approved[0].qty, 60);
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].0.qty, 50);
    assert!(matches!(out.rejected[0].1, Rejection::WeightCap));
}

// R-181 (a rejected order leaves the position unchanged)
#[test]
fn rejected_orders_do_not_change_the_position_seen_by_later_orders() {
    let out = run(
        &[order(Side::Buy, 101), order(Side::Buy, 100)],
        0,
        100000,
        1_000_000,
    );
    assert_eq!(out.approved.len(), 1);
    assert_eq!(out.approved[0].qty, 100);
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].0.qty, 101);
}

// R-181 (OrderNotional is checked before WeightCap)
#[test]
fn order_notional_is_reported_before_weight_cap() {
    let out = run(&[order(Side::Buy, 101)], 0, 100000, 10000);
    assert!(matches!(out.rejected[0].1, Rejection::OrderNotional));
}

// R-181 (NoEquity is checked before WeightCap)
#[test]
fn no_equity_is_reported_before_weight_cap() {
    let out = run(&[order(Side::Buy, 101)], 0, 0, 1_000_000);
    assert!(matches!(out.rejected[0].1, Rejection::NoEquity));
}
