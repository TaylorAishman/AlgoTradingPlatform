use std::collections::BTreeMap;

use pt_core::money::{Money, Price};
use pt_core::types::{Fill, Side, Symbol};
use pt_engine::book::Book;
use pt_engine::error::EngineError;
use rust_decimal::Decimal;

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn fill(side: Side, qty: u64, price: i64, cost: i64) -> Fill {
    Fill {
        symbol: sym("A"),
        side,
        qty,
        price: Price::new(dec(price)).unwrap(),
        cost: Money(dec(cost)),
    }
}

fn book_after_buy_and_sell() -> Book {
    let mut book = Book::new(dec(10000));
    book.apply_fill(&fill(Side::Buy, 10, 100, 0));
    book.apply_fill(&fill(Side::Sell, 15, 110, 0));
    book
}

// R-193
#[test]
fn buy_then_larger_sell_leaves_one_net_short_position_and_cash_10650() {
    let book = book_after_buy_and_sell();
    let expected = BTreeMap::from([(sym("A"), -5_i64)]);
    assert_eq!(book.positions, expected);
    assert_eq!(book.cash, dec(10650));
}

// R-193
#[test]
fn fill_cost_lowers_cash_by_a_further_amount() {
    let mut book = Book::new(dec(10000));
    book.apply_fill(&fill(Side::Buy, 10, 100, 2));
    assert_eq!(book.cash, dec(8998));
}

// R-193
#[test]
fn buying_back_to_flat_removes_the_position_entry() {
    let mut book = book_after_buy_and_sell();
    book.apply_fill(&fill(Side::Buy, 5, 100, 0));
    assert!(!book.positions.contains_key(&sym("A")));
    assert!(book.positions.is_empty());
}

// R-193
#[test]
fn equity_is_cash_plus_position_at_close() {
    let book = book_after_buy_and_sell();
    let closes = BTreeMap::from([(sym("A"), Price::new(dec(100)).unwrap())]);
    assert_eq!(book.equity(&closes), Ok(dec(10150)));
}

// R-193
#[test]
fn equity_without_a_close_for_a_held_symbol_is_missing_bar() {
    let book = book_after_buy_and_sell();
    let result = book.equity(&BTreeMap::new());
    assert!(
        matches!(result, Err(EngineError::MissingBar { ref symbol, .. }) if symbol == "A"),
        "got {result:?}"
    );
}

// R-193
#[test]
fn equity_of_an_empty_book_is_its_cash_without_any_closes() {
    let book = Book::new(dec(10000));
    assert_eq!(book.equity(&BTreeMap::new()), Ok(dec(10000)));
}
