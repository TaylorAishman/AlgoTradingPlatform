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

fn limits() -> RiskLimits {
    RiskLimits {
        max_order_notional: Decimal::new(1_000_000, 0),
        max_weight: Decimal::new(1, 0),
        max_gross: Decimal::new(1, 0),
        daily_loss_limit: Decimal::new(3, 2),
        max_drawdown: Decimal::new(15, 2),
    }
}

fn order(name: &str, side: Side, qty: u64, price: i64) -> OrderIntent {
    OrderIntent::new(sym(name), side, qty, 1, px(price)).unwrap()
}

fn run(orders: &[OrderIntent]) -> GateOutcome {
    let mut positions = Positions::new();
    positions.insert(sym("A"), 600);
    let prices = BTreeMap::from([(sym("A"), px(100)), (sym("B"), px(50))]);
    check_orders(
        orders,
        &positions,
        Money(Decimal::new(100_000, 0)),
        &prices,
        &limits(),
    )
}

// R-182
#[test]
fn orders_pushing_projected_gross_over_the_cap_are_both_rejected() {
    let out = run(&[
        order("A", Side::Buy, 300, 100),
        order("B", Side::Sell, 1000, 50),
    ]);
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 2);
    assert!(out
        .rejected
        .iter()
        .all(|(_, r)| matches!(r, Rejection::Gross)));
}

// R-182
#[test]
fn order_keeping_projected_gross_within_the_cap_is_approved() {
    let out = run(&[order("A", Side::Buy, 300, 100)]);
    assert_eq!(out.approved.len(), 1);
    assert!(out.rejected.is_empty());
}

// R-182: a symbol missing from the price map must not understate gross.
fn run_with_prices(
    held: &[(&str, i64)],
    prices: &[(&str, i64)],
    orders: &[OrderIntent],
) -> GateOutcome {
    let mut positions = Positions::new();
    for (name, qty) in held {
        positions.insert(sym(name), *qty);
    }
    let prices: BTreeMap<Symbol, Price> = prices.iter().map(|(n, p)| (sym(n), px(*p))).collect();
    check_orders(
        orders,
        &positions,
        Money(Decimal::new(100_000, 0)),
        &prices,
        &limits(),
    )
}

// R-182
#[test]
fn ordered_symbol_missing_from_prices_is_valued_at_its_reference_price() {
    // B has no market price; its 50 reference price gives 1500 * 50 = 75000,
    // plus A 600 * 100 = 60000: gross 135000 > 100000.
    let out = run_with_prices(
        &[("A", 600)],
        &[("A", 100)],
        &[order("B", Side::Buy, 1500, 50)],
    );
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].1, Rejection::Gross);
}

// R-182
#[test]
fn ordered_symbol_missing_from_prices_within_the_cap_is_approved() {
    let out = run_with_prices(
        &[("A", 600)],
        &[("A", 100)],
        &[order("B", Side::Buy, 100, 50)],
    );
    assert_eq!(out.approved.len(), 1);
    assert!(out.rejected.is_empty());
}

// R-182
#[test]
fn held_symbol_missing_from_prices_rejects_the_batch_as_gross() {
    // A is held but unpriced, so gross cannot be computed: fail closed.
    let out = run_with_prices(
        &[("A", 600)],
        &[("B", 50)],
        &[order("B", Side::Buy, 10, 50)],
    );
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 1);
    assert_eq!(out.rejected[0].1, Rejection::Gross);
}
