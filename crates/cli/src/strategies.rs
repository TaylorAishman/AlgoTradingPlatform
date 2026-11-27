//! Strategies the commands share.

use std::sync::Arc;

use anyhow::Context;
use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;
use pt_store::db::Store;
use pt_strategy::error::StrategyError;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};

/// Time-series momentum on the calendar in the store at the time of each call.
///
/// The cycle syncs the calendar before it decides, so a fresh store has no
/// calendar when the run starts; reading it per call picks up that sync.
pub struct StoredTsmom<'a> {
    store: &'a Store,
    params: TsmomParams,
    /// Answers `id` and `params_canonical`, which do not read the calendar.
    template: Tsmom,
}

impl<'a> StoredTsmom<'a> {
    pub fn new(store: &'a Store, params: TsmomParams, start: NaiveDate) -> anyhow::Result<Self> {
        let placeholder = TradingCalendar::new(vec![start]).context("strategy calendar")?;
        let template = Tsmom::new(params.clone(), Arc::new(placeholder));
        Ok(Self {
            store,
            params,
            template,
        })
    }
}

impl Strategy for StoredTsmom<'_> {
    fn id(&self) -> &str {
        self.template.id()
    }

    fn params_canonical(&self) -> String {
        self.template.params_canonical()
    }

    fn signals(
        &self,
        universe: &[Symbol],
        asof: NaiveDate,
        view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        let calendar = self
            .store
            .calendar()
            .map_err(|e| StrategyError::Invalid(format!("reading the calendar: {e}")))?;
        Tsmom::new(self.params.clone(), Arc::new(calendar)).signals(universe, asof, view)
    }
}
