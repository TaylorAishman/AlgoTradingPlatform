//! The shared decision function and its parameters.

use crate::error::EngineError;
use chrono::NaiveDate;
use pt_config::config::Config;
use pt_core::money::{Money, Price};
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{OrderIntent, Positions, Symbol, Weights};
use pt_portfolio::orders::{plan_orders, target_shares};
use pt_portfolio::weights::{target_weights, SizingParams};
use pt_risk::pretrade::{Rejection, RiskLimits};
use pt_strategy::estimate::VolParams;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};

/// What a decision is made from: the dates, the account and the universe.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionInputs {
    pub signal_date: NaiveDate,
    pub trade_date: NaiveDate,
    pub ref_date: NaiveDate,
    pub equity: Money,
    pub positions: Positions,
    pub universe: Vec<Symbol>,
    pub shortable: BTreeSet<Symbol>,
}

/// A decision: what it was given and the weights, targets and orders it produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub signals: SignalSet,
    pub risk: RiskEstimates,
    pub weights: Weights,
    pub target: Positions,
    pub orders: Vec<OrderIntent>,
    pub rejected: Vec<(OrderIntent, Rejection)>,
}

/// Everything the decision function reads from configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionParams {
    pub vol: VolParams,
    pub sizing: SizingParams,
    pub limits: RiskLimits,
}

fn float(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(f64::NAN)
}

/// The one place config becomes decision parameters.
pub fn params_from_config(cfg: &Config) -> DecisionParams {
    DecisionParams {
        vol: VolParams {
            com_days: f64::from(cfg.strategy.vol_com_days),
            annualization: f64::from(cfg.strategy.vol_annualization),
            min_obs: cfg.strategy.vol_min_obs as usize,
            max_obs: cfg.strategy.vol_max_obs as usize,
        },
        sizing: SizingParams {
            target_vol: float(cfg.sizing.target_vol),
            max_weight: float(cfg.sizing.max_weight),
            max_gross: float(cfg.sizing.max_gross),
        },
        limits: RiskLimits {
            max_order_notional: cfg.risk.max_order_notional,
            max_weight: cfg.sizing.max_weight,
            max_gross: cfg.sizing.max_gross,
            daily_loss_limit: cfg.risk.daily_loss_limit,
            max_drawdown: cfg.risk.max_drawdown,
        },
    }
}

/// Turns signals and risk estimates into weights (Arch 5.10.3), whole-share
/// targets and orders (Arch 5.10.4) at the given reference prices.
pub fn decide_with(
    signals: SignalSet,
    risk: RiskEstimates,
    inputs: &DecisionInputs,
    ref_prices: &BTreeMap<Symbol, Price>,
    p: &DecisionParams,
) -> Result<Decision, EngineError> {
    let weights = target_weights(&signals.signals, &risk, &inputs.shortable, &p.sizing);
    let target = target_shares(&weights, inputs.equity, ref_prices)?;
    let orders = plan_orders(&inputs.positions, &target, ref_prices)?;
    Ok(Decision {
        signals,
        risk,
        weights,
        target,
        orders,
        rejected: Vec::new(),
    })
}
