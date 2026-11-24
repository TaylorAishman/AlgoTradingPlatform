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

fn prices(names: &[&str]) -> BTreeMap<Symbol, Price> {
    names
        .iter()
        .map(|n| (sym(n), Price::new(Decimal::new(100, 0)).unwrap()))
        .collect()
}

fn summary(items: &[pt_core::types::OrderIntent]) -> Vec<(String, Side, u64, u8)> {
    items
        .iter()
        .map(|o| (o.symbol.as_str().to_string(), o.side, o.qty, o.leg))
        .collect()
}

// R-169
#[test]
fn sells_come_before_buys_and_closing_legs_before_opening_legs() {
    let orders = plan_orders(
        &positions(&[("B", 10), ("C", -5), ("D", 20)]),
        &positions(&[("A", 5), ("B", 0), ("C", 5), ("D", -5)]),
        &prices(&["A", "B", "C", "D"]),
    )
    .unwrap();
    let s = |n: &str, side, qty, leg| (n.to_string(), side, qty, leg);
    assert_eq!(
        summary(&orders),
        vec![
            s("B", Side::Sell, 10, 1),
            s("D", Side::Sell, 20, 1),
            s("A", Side::Buy, 5, 1),
            s("C", Side::Buy, 5, 1),
            s("D", Side::Sell, 5, 2),
            s("C", Side::Buy, 5, 2),
        ]
    );
}

// R-169
#[test]
fn a_buy_for_an_earlier_symbol_follows_a_sell_for_a_later_symbol() {
    let orders = plan_orders(
        &positions(&[("Z", 3)]),
        &positions(&[("A", 4)]),
        &prices(&["A", "Z"]),
    )
    .unwrap();
    let s = summary(&orders);
    assert_eq!(s[0].0, "Z");
    assert_eq!(s[0].1, Side::Sell);
    assert_eq!(s[1].0, "A");
    assert_eq!(s[1].1, Side::Buy);
}
