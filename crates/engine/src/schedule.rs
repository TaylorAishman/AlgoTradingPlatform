//! The weight schedule and the backtest loop.

use crate::decision::DecisionParams;
use crate::error::EngineError;
use crate::simulate::{simulate, SimConfig, SimResult};
use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_calendar::schedule::Rebalance;
use pt_core::types::{Symbol, Weights};
use pt_core::view::HistoryView;
use pt_core::view::MarketView;
use pt_portfolio::weights::target_weights;
use pt_strategy::estimate::estimate_risk;
use pt_strategy::strategy::Strategy;
use std::collections::{BTreeMap, BTreeSet};

/// Target weights for each rebalance, keyed by trade date. Each entry reads
/// data up to its signal date only.
pub fn weights_schedule(
    schedule: &[Rebalance],
    universe: &[Symbol],
    shortable: &BTreeSet<Symbol>,
    strategy: &dyn Strategy,
    view: &dyn HistoryView,
    p: &DecisionParams,
) -> Result<BTreeMap<NaiveDate, Weights>, EngineError> {
    let mut out = BTreeMap::new();
    for r in schedule {
        let signals = strategy.signals(universe, r.signal_date, view)?;
        let risk = estimate_risk(universe, r.signal_date, view, &p.vol);
        let weights = target_weights(&signals.signals, &risk, shortable, &p.sizing);
        out.insert(r.trade_date, weights);
    }
    Ok(out)
}

/// A backtest: the simulation of the strategy's own weight schedule.
pub fn run_backtest(
    cfg: &SimConfig,
    schedule: &[Rebalance],
    shortable: &BTreeSet<Symbol>,
    strategy: &dyn Strategy,
    view: &dyn MarketView,
    cal: &TradingCalendar,
    p: &DecisionParams,
) -> Result<SimResult, EngineError> {
    let weights = weights_schedule(schedule, &cfg.universe, shortable, strategy, view, p)?;
    simulate(cfg, &weights, view, cal)
}
