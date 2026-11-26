//! The buy-and-hold benchmark: long every instrument that has a price.

use crate::error::StrategyError;
use crate::strategy::Strategy;
use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The strategy id.
pub const ID: &str = "bh-volscaled-v1";

/// Signals +1.0 for every symbol with an adjusted close on the decision date.
#[derive(Debug, Clone)]
pub struct LongOnly {
    calendar: Arc<TradingCalendar>,
}

impl LongOnly {
    pub fn new(cal: Arc<TradingCalendar>) -> LongOnly {
        LongOnly { calendar: cal }
    }
}

impl Strategy for LongOnly {
    fn id(&self) -> &str {
        ID
    }

    fn params_canonical(&self) -> String {
        String::new()
    }

    fn signals(
        &self,
        universe: &[Symbol],
        asof: NaiveDate,
        view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        if !self.calendar.is_session(asof) {
            return Err(StrategyError::NotASession(asof));
        }
        let mut signals = BTreeMap::new();
        let mut ineligible = BTreeMap::new();
        for symbol in universe {
            let priced = view
                .adj_closes(symbol, asof, 1)
                .last()
                .is_some_and(|(d, _)| *d == asof);
            if priced {
                signals.insert(symbol.clone(), 1.0);
            } else {
                ineligible.insert(symbol.clone(), "no_price".to_string());
            }
        }
        Ok(SignalSet {
            asof,
            signals,
            ineligible,
        })
    }
}
