use pt_core::money::{Money, Price};
use pt_core::types::{Positions, Symbol, Weights};
use pt_portfolio::orders::target_shares;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn shares_for(equity: &str, weight: &str, price: &str) -> i64 {
    let mut weights = Weights::new();
    weights.insert(sym("A"), d(weight));
    let mut prices = BTreeMap::new();
    prices.insert(sym("A"), Price::new(d(price)).unwrap());
    let out: Positions = target_shares(&weights, Money(d(equity)), &prices).unwrap();
    out[&sym("A")]
}

// R-165
#[test]
fn positive_weight_gives_whole_shares() {
    assert_eq!(shares_for("100000", "0.5", "100"), 500);
}

// R-165
#[test]
fn negative_weight_gives_negative_shares() {
    assert_eq!(shares_for("100000", "-0.25", "50"), -500);
}

// R-165
#[test]
fn positive_fraction_truncates_toward_zero() {
    assert_eq!(shares_for("100000", "0.1", "333.33"), 30);
}

// R-165
#[test]
fn negative_fraction_truncates_toward_zero() {
    assert_eq!(shares_for("100000", "-0.1", "333.33"), -30);
}

// R-165
#[test]
fn short_with_larger_equity_truncates_toward_zero() {
    assert_eq!(shares_for("101000", "-0.2", "102"), -198);
}

// R-165
#[test]
fn each_symbol_gets_its_own_target() {
    let mut weights = Weights::new();
    weights.insert(sym("A"), d("0.5"));
    weights.insert(sym("B"), d("-0.25"));
    let mut prices = BTreeMap::new();
    prices.insert(sym("A"), Price::new(d("100")).unwrap());
    prices.insert(sym("B"), Price::new(d("50")).unwrap());
    let out = target_shares(&weights, Money(d("100000")), &prices).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[&sym("A")], 500);
    assert_eq!(out[&sym("B")], -500);
}
