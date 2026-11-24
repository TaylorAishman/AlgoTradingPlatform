//! Core domain types.

use std::collections::BTreeMap;
use std::fmt;

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::error::CoreError;
use crate::money::{Money, Price};

/// Longest accepted symbol, in characters.
const MAX_SYMBOL_LEN: usize = 12;

/// A validated ticker symbol: 1 to 12 characters of `A-Z`, `0-9` and `.`,
/// starting with a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(String);

impl Symbol {
    /// Validate and wrap a symbol.
    pub fn new(s: &str) -> Result<Self, CoreError> {
        let valid = s.len() <= MAX_SYMBOL_LEN
            && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && s.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.');
        if valid {
            Ok(Self(s.to_string()))
        } else {
            Err(CoreError::InvalidSymbol(s.to_string()))
        }
    }

    /// The symbol text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Order direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    /// Buy is +1, Sell is -1.
    pub fn sign(&self) -> i64 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
}

/// Alpaca `adjustment` values "raw" and "all".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Adjustment {
    Raw,
    All,
}

/// Which price series the engine fills and marks on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriceBasis {
    Raw,
    Adjusted,
}

/// A daily bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bar {
    pub symbol: Symbol,
    pub date: NaiveDate,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: u64,
}

/// A validated order intent: positive quantity, leg 1 or 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIntent {
    pub symbol: Symbol,
    pub side: Side,
    pub qty: u64,
    pub leg: u8,
    pub reference_price: Price,
}

impl OrderIntent {
    /// Validate and build an order intent.
    pub fn new(
        symbol: Symbol,
        side: Side,
        qty: u64,
        leg: u8,
        reference_price: Price,
    ) -> Result<OrderIntent, CoreError> {
        if qty == 0 {
            return Err(CoreError::InvalidOrder("quantity must be positive".into()));
        }
        if !(1..=2).contains(&leg) {
            return Err(CoreError::InvalidOrder(format!(
                "leg must be 1 or 2, got {leg}"
            )));
        }
        Ok(OrderIntent {
            symbol,
            side,
            qty,
            leg,
            reference_price,
        })
    }
}

/// A simulated or real fill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fill {
    pub symbol: Symbol,
    pub side: Side,
    pub qty: u64,
    pub price: Price,
    pub cost: Money,
}

/// Target weights per symbol, 6 decimal places.
pub type Weights = BTreeMap<Symbol, Decimal>;
/// Signed whole shares, one entry per symbol.
pub type Positions = BTreeMap<Symbol, i64>;
