//! The store-backed market view.

use crate::error::DataError;
use chrono::{DateTime, NaiveDate, Utc};
use pt_core::types::{Adjustment, PriceBasis, Symbol};
use pt_core::view::HistoryView;
use pt_store::db::Store;
use rust_decimal::prelude::ToPrimitive;
use std::collections::BTreeMap;

/// Market data read from the store once, as of an optional instant.
#[derive(Debug, Clone)]
pub struct StoreView {
    adj: BTreeMap<Symbol, BTreeMap<NaiveDate, f64>>,
}

impl StoreView {
    /// Load the fully adjusted closes of `symbols` from `feed`, using each date's latest
    /// version fetched at or before `asof` (the latest overall when `None`).
    pub fn load(
        store: &Store,
        symbols: &[Symbol],
        feed: &str,
        _basis: PriceBasis,
        asof: Option<DateTime<Utc>>,
    ) -> Result<StoreView, DataError> {
        let mut adj = BTreeMap::new();
        for symbol in symbols {
            let bars = store.bars_asof(
                symbol,
                Adjustment::All,
                feed,
                NaiveDate::from_ymd_opt(1, 1, 1).unwrap_or(NaiveDate::MIN),
                NaiveDate::from_ymd_opt(9999, 12, 31).unwrap_or(NaiveDate::MAX),
                asof,
            )?;
            let mut series = BTreeMap::new();
            for bar in bars {
                let close = bar.close.value().to_f64().ok_or_else(|| {
                    DataError::Invalid(format!("close of {symbol} on {} is not a number", bar.date))
                })?;
                series.insert(bar.date, close);
            }
            adj.insert(symbol.clone(), series);
        }
        Ok(StoreView { adj })
    }
}

impl HistoryView for StoreView {
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
