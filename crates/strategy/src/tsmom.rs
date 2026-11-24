//! Time-series momentum: long when the trailing return beats the risk-free return, short when not.

use crate::error::StrategyError;
use crate::strategy::Strategy;
use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Parameters of [`Tsmom`].
#[derive(Debug, Clone, PartialEq)]
pub struct TsmomParams {
    pub lookback_months: Vec<u32>,
    pub risk_free_symbol: Symbol,
}

/// Time-series momentum on month-end adjusted closes.
#[derive(Debug, Clone)]
pub struct Tsmom {
    params: TsmomParams,
    calendar: Arc<TradingCalendar>,
}

impl Tsmom {
    pub fn new(p: TsmomParams, cal: Arc<TradingCalendar>) -> Tsmom {
        Tsmom {
            params: p,
            calendar: cal,
        }
    }
}

/// The adjusted close on exactly `date`, if there is one.
fn close_on(view: &dyn HistoryView, symbol: &Symbol, date: NaiveDate) -> Option<f64> {
    view.adj_closes(symbol, date, 1)
        .last()
        .filter(|(d, _)| *d == date)
        .map(|&(_, c)| c)
}

impl Strategy for Tsmom {
    fn id(&self) -> &str {
        "tsmom-v1"
    }

    fn params_canonical(&self) -> String {
        let months: Vec<String> = self
            .params
            .lookback_months
            .iter()
            .map(u32::to_string)
            .collect();
        format!(
            "lookback_months={};risk_free={}",
            months.join(","),
            self.params.risk_free_symbol
        )
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
        let rf = &self.params.risk_free_symbol;
        let rf_now = close_on(view, rf, asof).ok_or(StrategyError::MissingRiskFree(asof))?;
        let mut signals = BTreeMap::new();
        let mut ineligible = BTreeMap::new();
        for symbol in universe {
            let now = close_on(view, symbol, asof);
            let mut total = 0.0;
            let mut missing = None;
            for &months in &self.params.lookback_months {
                let Some(anchor) = self.calendar.month_end_back(asof, months) else {
                    missing = Some(format!("no session {months} months back"));
                    break;
                };
                let (Some(now), Some(then), Some(rf_then)) = (
                    now,
                    close_on(view, symbol, anchor),
                    close_on(view, rf, anchor),
                ) else {
                    missing = Some(format!("no price on {anchor}"));
                    break;
                };
                let excess = (now / then - 1.0) - (rf_now / rf_then - 1.0);
                total += if excess > 0.0 {
                    1.0
                } else if excess < 0.0 {
                    -1.0
                } else {
                    0.0
                };
            }
            match missing {
                Some(reason) => {
                    ineligible.insert(symbol.clone(), reason);
                }
                None => {
                    let n = self.params.lookback_months.len().max(1) as f64;
                    signals.insert(symbol.clone(), total / n);
                }
            }
        }
        Ok(SignalSet {
            asof,
            signals,
            ineligible,
        })
    }
}
