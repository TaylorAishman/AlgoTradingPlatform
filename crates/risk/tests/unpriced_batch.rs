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

fn run(prices: &[(&str, i64)]) -> GateOutcome {
    let mut positions = Positions::new();
    positions.insert(sym("SPY"), 10);
    positions.insert(sym("GLD"), 3);
    let prices: BTreeMap<Symbol, Price> = prices.iter().map(|(n, p)| (sym(n), px(*p))).collect();
    let orders = [
        OrderIntent::new(sym("SPY"), Side::Sell, 5, 1, px(100)).unwrap(),
        OrderIntent::new(sym("SPY"), Side::Buy, 1, 1, px(100)).unwrap(),
    ];
    check_orders(
        &orders,
        &positions,
        Money(Decimal::new(100_000, 0)),
        &prices,
        &limits(),
    )
}

// R-453
#[test]
fn batch_with_an_unpriced_held_symbol_is_rejected_whole_including_reducing_orders() {
    let out = run(&[("SPY", 100)]);
    assert!(out.approved.is_empty());
    assert_eq!(out.rejected.len(), 2);
    assert!(out.rejected.iter().all(|(_, r)| *r == Rejection::Gross));
}

// R-453
#[test]
fn reducing_sell_is_approved_once_every_held_symbol_is_priced() {
    let out = run(&[("SPY", 100), ("GLD", 50)]);
    assert_eq!(out.approved.len(), 2);
    assert!(out.rejected.is_empty());
    assert_eq!(out.approved[0].side, Side::Sell);
}
