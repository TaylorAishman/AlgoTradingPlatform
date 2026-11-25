//! The shared decision function and its parameters.

use pt_config::config::Config;
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::estimate::VolParams;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

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
