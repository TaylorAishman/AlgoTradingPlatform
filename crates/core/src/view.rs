//! Read-only market data views.

use crate::types::{Bar, Symbol};
use chrono::NaiveDate;
use std::collections::BTreeMap;

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

/// In-memory view for tests and the vector cross-check.
#[derive(Debug, Clone, Default)]
pub struct MemoryView {
    adj: BTreeMap<Symbol, BTreeMap<NaiveDate, f64>>,
    exec: BTreeMap<Symbol, BTreeMap<NaiveDate, Bar>>,
}

impl MemoryView {
    pub fn new() -> MemoryView {
        MemoryView::default()
    }

    /// Record an execution bar; a later push for the same symbol and date replaces the earlier one.
    pub fn push_exec(&mut self, bar: Bar) {
        self.exec
            .entry(bar.symbol.clone())
            .or_default()
            .insert(bar.date, bar);
    }

    /// Record an adjusted close; a later push for the same date replaces the earlier one.
    pub fn push_adj_close(&mut self, symbol: &Symbol, date: NaiveDate, close: f64) {
        self.adj
            .entry(symbol.clone())
            .or_default()
            .insert(date, close);
    }
}

impl HistoryView for MemoryView {
    fn adj_closes(&self, symbol: &Symbol, asof: NaiveDate, max: usize) -> Vec<(NaiveDate, f64)> {
        let Some(series) = self.adj.get(symbol) else {
            return Vec::new();
        };
        let mut out: Vec<(NaiveDate, f64)> = series
            .range(..=asof)
            .rev()
            .take(max)
            .map(|(d, c)| (*d, *c))
            .collect();
        out.reverse();
        out
    }
}

impl MarketView for MemoryView {
    fn exec_bar(&self, symbol: &Symbol, date: NaiveDate) -> Option<Bar> {
        self.exec.get(symbol)?.get(&date).cloned()
    }

    /// Mean volume of the latest `sessions` bars on or before `asof`.
    fn adv_shares(&self, symbol: &Symbol, asof: NaiveDate, sessions: usize) -> Option<f64> {
        if sessions == 0 {
            return None;
        }
        let series = self.exec.get(symbol)?;
        let vols: Vec<f64> = series
            .range(..=asof)
            .rev()
            .take(sessions)
            .map(|(_, b)| b.volume as f64)
            .collect();
        if vols.len() < sessions {
            return None;
        }
        Some(vols.iter().sum::<f64>() / sessions as f64)
    }
}
