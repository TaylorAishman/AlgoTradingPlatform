//! The book: cash and net share positions, moved by fills.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pt_core::money::Price;
use pt_core::types::{Fill, Positions, Symbol};
use rust_decimal::Decimal;

use crate::error::EngineError;

/// Cash plus signed net positions per symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub cash: Decimal,
    pub positions: Positions,
}

impl Book {
    /// An empty book holding only `cash`.
    pub fn new(cash: Decimal) -> Self {
        Self {
            cash,
            positions: Positions::new(),
        }
    }

    /// Move cash by the signed notional, subtract the cost, and update the position.
    /// A position that goes flat is removed.
    pub fn apply_fill(&mut self, fill: &Fill) {
        let qty = i64::try_from(fill.qty).expect("fill quantity fits in i64");
        let signed = fill.side.sign() * qty;
        self.cash -= fill.price.notional(signed).0 + fill.cost.0;
        let entry = self.positions.entry(fill.symbol.clone()).or_insert(0);
        *entry += signed;
        if *entry == 0 {
            self.positions.remove(&fill.symbol);
        }
    }

    /// Cash plus every position at its close. A held symbol without a close is `MissingBar`.
    /// The error carries `NaiveDate::MIN`, since the book does not know the date.
    pub fn equity(&self, closes: &BTreeMap<Symbol, Price>) -> Result<Decimal, EngineError> {
        let mut total = self.cash;
        for (symbol, qty) in &self.positions {
            let close = closes.get(symbol).ok_or_else(|| EngineError::MissingBar {
                symbol: symbol.to_string(),
                date: NaiveDate::MIN,
            })?;
            total += close.notional(*qty).0;
        }
        Ok(total)
    }
}
