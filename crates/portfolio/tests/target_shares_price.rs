use pt_core::money::{Money, Price};
use pt_core::types::{Symbol, Weights};
use pt_portfolio::error::PortfolioError;
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

// R-166
#[test]
fn non_zero_weight_without_price_is_missing_price() {
    let mut weights = Weights::new();
    weights.insert(sym("A"), d("0.1"));
    let prices: BTreeMap<Symbol, Price> = BTreeMap::new();
    let err = target_shares(&weights, Money(d("100000")), &prices).unwrap_err();
    assert_eq!(err, PortfolioError::MissingPrice("A".to_string()));
}

// R-166
#[test]
fn negative_weight_without_price_is_missing_price() {
    let mut weights = Weights::new();
    weights.insert(sym("A"), d("-0.1"));
    let prices: BTreeMap<Symbol, Price> = BTreeMap::new();
    let err = target_shares(&weights, Money(d("100000")), &prices).unwrap_err();
    assert_eq!(err, PortfolioError::MissingPrice("A".to_string()));
}

// R-166
#[test]
fn zero_weight_without_price_gives_zero_and_no_error() {
    let mut weights = Weights::new();
    weights.insert(sym("A"), d("0"));
    let prices: BTreeMap<Symbol, Price> = BTreeMap::new();
    let out = target_shares(&weights, Money(d("100000")), &prices).unwrap();
    assert_eq!(out[&sym("A")], 0);
}

// R-166
#[test]
fn price_for_another_symbol_does_not_satisfy_a_non_zero_weight() {
    let mut weights = Weights::new();
    weights.insert(sym("A"), d("0.1"));
    weights.insert(sym("B"), d("0"));
    let mut prices = BTreeMap::new();
    prices.insert(sym("B"), Price::new(d("10")).unwrap());
    let err = target_shares(&weights, Money(d("100000")), &prices).unwrap_err();
    assert_eq!(err, PortfolioError::MissingPrice("A".to_string()));
}
