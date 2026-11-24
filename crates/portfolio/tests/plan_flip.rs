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
    m
}

// R-168
#[test]
fn long_to_short_is_a_closing_sell_then_an_opening_sell() {
    let orders = plan_orders(
        &positions(&[("A", 500)]),
        &positions(&[("A", -198)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 2);
    for o in &orders {
        assert_eq!(o.symbol, sym("A"));
        assert_eq!(o.side, Side::Sell);
        assert_eq!(o.reference_price, prices()[&sym("A")]);
    }
    assert_eq!((orders[0].qty, orders[0].leg), (500, 1));
    assert_eq!((orders[1].qty, orders[1].leg), (198, 2));
}

// R-168
#[test]
fn short_to_long_is_a_closing_buy_then_an_opening_buy() {
    let orders = plan_orders(
        &positions(&[("A", -100)]),
        &positions(&[("A", 50)]),
        &prices(),
    )
    .unwrap();
    assert_eq!(orders.len(), 2);
    for o in &orders {
        assert_eq!(o.side, Side::Buy);
    }
    assert_eq!((orders[0].qty, orders[0].leg), (100, 1));
    assert_eq!((orders[1].qty, orders[1].leg), (50, 2));
}

// R-168
#[test]
fn a_move_to_exactly_zero_is_not_a_flip() {
    let orders = plan_orders(&positions(&[("A", 5)]), &positions(&[("A", 0)]), &prices()).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].leg, 1);
}

// R-168
#[test]
fn a_flip_without_a_price_is_an_error() {
    let result = plan_orders(
        &positions(&[("A", 5)]),
        &positions(&[("A", -5)]),
        &BTreeMap::new(),
    );
    assert!(result.is_err());
}
