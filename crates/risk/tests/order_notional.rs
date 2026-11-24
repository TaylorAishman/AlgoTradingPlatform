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
        max_weight: Decimal::new(10, 1),
        max_gross: Decimal::new(10, 1),
        daily_loss_limit: Decimal::new(3, 2),
        max_drawdown: Decimal::new(15, 2),
    }
}

fn order(s: &str, side: Side, qty: u64) -> OrderIntent {
    OrderIntent::new(sym(s), side, qty, 1, px(100)).unwrap()
}

fn prices() -> BTreeMap<Symbol, Price> {
    BTreeMap::from([(sym("A"), px(100))])
}

fn equity(n: i64) -> Money {
    Money(Decimal::new(n, 0))
}

// R-180
#[test]
fn order_at_the_maximum_notional_is_approved() {
    let out = check_orders(
        &[order("A", Side::Buy, 250)],
        &Positions::new(),
        equity(100000),
        &prices(),
        &limits(25000),
    );
    assert_eq!(out.approved.len(), 1);
    assert_eq!(out.approved[0].qty, 250);
    assert!(out.rejected.is_empty());
}

// R-180
#[test]
fn order_above_the_maximum_notional_is_rejected() {
    let out = check_orders(
        &[order("A", Side::Buy, 251)],
        &Positions::new(),
        equity(100000),
        &prices(),
        &limits(25000),
    );
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].0.qty, 251);
    assert!(matches!(out.rejected[0].1, Rejection::OrderNotional));
}

// R-180
#[test]
fn sell_order_above_the_maximum_notional_is_rejected() {
    let out = check_orders(
        &[order("A", Side::Sell, 251)],
        &Positions::new(),
        equity(100000),
        &prices(),
        &limits(25000),
    );
    assert!(out.approved.is_empty());
    assert!(matches!(out.rejected[0].1, Rejection::OrderNotional));
}

// R-180
#[test]
fn each_order_is_judged_on_its_own_notional() {
    let out = check_orders(
        &[order("A", Side::Buy, 251), order("A", Side::Buy, 10)],
        &Positions::new(),
        equity(100000),
        &prices(),
        &limits(25000),
    );
    assert_eq!(out.approved.len(), 1);
    assert_eq!(out.approved[0].qty, 10);
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].0.qty, 251);
}

// R-184
#[test]
fn zero_equity_rejects_every_order_as_no_equity() {
    let out = check_orders(
        &[order("A", Side::Buy, 1), order("A", Side::Sell, 2)],
        &Positions::new(),
        equity(0),
        &prices(),
        &limits(25000),
    );
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 2);
    assert!(out
        .rejected
        .iter()
        .all(|(_, r)| matches!(r, Rejection::NoEquity)));
}

// R-184
#[test]
fn negative_equity_rejects_every_order_as_no_equity() {
    let out = check_orders(
        &[order("A", Side::Buy, 1)],
        &Positions::new(),
        equity(-1),
        &prices(),
        &limits(25000),
    );
    assert!(out.approved.is_empty());
    assert!(matches!(out.rejected[0].1, Rejection::NoEquity));
}

// R-184, R-181 (NoEquity is checked before OrderNotional)
#[test]
fn no_equity_is_reported_before_order_notional() {
    let out = check_orders(
        &[order("A", Side::Buy, 251)],
        &Positions::new(),
        equity(0),
        &prices(),
        &limits(25000),
    );
    assert!(matches!(out.rejected[0].1, Rejection::NoEquity));
}
