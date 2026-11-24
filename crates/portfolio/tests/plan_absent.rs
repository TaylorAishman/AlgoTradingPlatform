use pt_core::money::Price;
use pt_core::types::{Positions, Side, Symbol};
use pt_portfolio::orders::plan_orders;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn positions(items: &[(&str, i64)]) -> Positions {
    items.iter().map(|(s, q)| (sym(s), *q)).collect()
}

fn prices() -> BTreeMap<Symbol, Price> {
    let mut m = BTreeMap::new();
    m.insert(sym("A"), Price::new(Decimal::new(100, 0)).unwrap());
    m.insert(sym("B"), Price::new(Decimal::new(50, 0)).unwrap());
    m
}

// R-170
#[test]
fn a_short_symbol_missing_from_the_target_is_bought_back() {
    let orders = plan_orders(
        &positions(&[("A", 10), ("B", -4)]),
        &positions(&[("A", 10)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].symbol, sym("B"));
    assert_eq!(orders[0].side, Side::Buy);
    assert_eq!(orders[0].qty, 4);
    assert_eq!(orders[0].leg, 1);
    assert_eq!(orders[0].reference_price, prices()[&sym("B")]);
}

// R-170
#[test]
fn a_long_symbol_missing_from_the_target_is_sold() {
    let orders = plan_orders(&positions(&[("A", 10)]), &positions(&[]), &prices()).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].symbol, sym("A"));
    assert_eq!(orders[0].side, Side::Sell);
    assert_eq!(orders[0].qty, 10);
    assert_eq!(orders[0].leg, 1);
}

// R-170
#[test]
fn a_zero_holding_missing_from_the_target_gives_no_order() {
    let orders = plan_orders(&positions(&[("A", 0)]), &positions(&[]), &prices()).unwrap();
    assert!(orders.is_empty());
}

// R-170
#[test]
fn a_held_symbol_missing_from_the_target_and_prices_is_an_error() {
    let result = plan_orders(&positions(&[("C", 5)]), &positions(&[]), &prices());
    assert!(result.is_err());
}
