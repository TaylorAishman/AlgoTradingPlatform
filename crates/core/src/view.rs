//! Read-only market data views.

use crate::types::{Bar, Symbol};
use chrono::NaiveDate;

pub trait HistoryView {
    /// Adjusted closes for sessions on or before `asof`, oldest first, at most `max` of them.
    fn adj_closes(&self, symbol: &Symbol, asof: NaiveDate, max: usize) -> Vec<(NaiveDate, f64)>;
}

pub trait MarketView: HistoryView {
    /// The bar on the execution basis for `date`, if any.
    fn exec_bar(&self, symbol: &Symbol, date: NaiveDate) -> Option<Bar>;
    /// Mean volume in shares over the `sessions` sessions before `asof`.
    fn adv_shares(&self, symbol: &Symbol, asof: NaiveDate, sessions: usize) -> Option<f64>;
}
