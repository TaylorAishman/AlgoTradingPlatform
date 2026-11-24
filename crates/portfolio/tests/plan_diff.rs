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
    m.insert(sym("A"), Price::new(Decimal::new(10025, 2)).unwrap());
    m.insert(sym("B"), Price::new(Decimal::new(50, 0)).unwrap());
    m
}

// R-167
#[test]
fn empty_current_and_positive_target_buys_the_target() {
    let orders = plan_orders(&positions(&[]), &positions(&[("A", 500)]), &prices()).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].symbol, sym("A"));
    assert_eq!(orders[0].side, Side::Buy);
    assert_eq!(orders[0].qty, 500);
    assert_eq!(orders[0].leg, 1);
}

// R-167
#[test]
fn reducing_a_long_position_sells_the_difference() {
    let orders = plan_orders(
        &positions(&[("A", 500)]),
        &positions(&[("A", 300)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].side, Side::Sell);
    assert_eq!(orders[0].qty, 200);
    assert_eq!(orders[0].leg, 1);
}

// R-167
#[test]
fn equal_current_and_target_gives_no_order() {
    let orders = plan_orders(
        &positions(&[("A", 500)]),
        &positions(&[("A", 500)]),
        &prices(),
    )
    .unwrap();
    assert!(orders.is_empty());
}

// R-167
#[test]
fn growing_a_short_position_sells_the_difference() {
    let orders = plan_orders(
        &positions(&[("A", -100)]),
        &positions(&[("A", -250)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].side, Side::Sell);
    assert_eq!(orders[0].qty, 150);
    assert_eq!(orders[0].leg, 1);
}

// R-167
#[test]
fn covering_part_of_a_short_position_buys_the_difference() {
    let orders = plan_orders(
        &positions(&[("A", -100)]),
        &positions(&[("A", -40)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].side, Side::Buy);
    assert_eq!(orders[0].qty, 60);
    assert_eq!(orders[0].leg, 1);
}

// R-167
#[test]
fn each_order_carries_the_reference_price_of_its_symbol() {
    let orders = plan_orders(
        &positions(&[]),
        &positions(&[("A", 10), ("B", 20)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 2);
    for o in &orders {
        assert_eq!(o.reference_price, prices()[&o.symbol]);
    }
    let a = orders.iter().find(|o| o.symbol == sym("A")).unwrap();
    let b = orders.iter().find(|o| o.symbol == sym("B")).unwrap();
    assert_eq!(a.reference_price.value(), Decimal::new(10025, 2));
    assert_eq!(b.reference_price.value(), Decimal::new(50, 0));
}
