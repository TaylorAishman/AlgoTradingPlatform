//! Risk estimates from adjusted closes (Arch 5.10.1).

use std::collections::BTreeMap;

use chrono::NaiveDate;
pub use pt_core::signal::RiskEstimates;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;

use crate::vol::{ewma_cov, ewma_vol};

/// Parameters of the volatility estimate.
#[derive(Debug, Clone, PartialEq)]
pub struct VolParams {
    pub com_days: f64,
    pub annualization: f64,
    pub min_obs: usize,
    pub max_obs: usize,
}

/// Estimate volatility for each symbol from closes on or before `asof`.
///
/// Uses at most the last `max_obs` returns. A symbol with fewer than
/// `min_obs` returns is ineligible with reason `insufficient_history`. A
/// symbol missing a session that another symbol has inside the shared window
/// is ineligible with reason `gap`; non-finite volatility gives
/// `non_finite`; zero volatility gives `zero_vol`.
/// `cov` holds every ordered pair of eligible symbols over the same returns.
pub fn estimate_risk(
    symbols: &[Symbol],
    asof: NaiveDate,
    view: &dyn HistoryView,
    p: &VolParams,
) -> RiskEstimates {
    let mut risk = RiskEstimates {
        asof,
        vols: BTreeMap::new(),
        cov: BTreeMap::new(),
        ineligible: BTreeMap::new(),
    };
    let n = p.max_obs + 1;
    let mut calendar: Vec<NaiveDate> = symbols
        .iter()
        .flat_map(|s| view.adj_closes(s, asof, n))
        .map(|(d, _)| d)
        .collect();
    calendar.sort();
    calendar.dedup();
    let window = &calendar[calendar.len().saturating_sub(n)..];
    let mut eligible_returns: Vec<(Symbol, Vec<f64>)> = Vec::new();
    for s in symbols {
        let closes = view.adj_closes(s, asof, n);
        let returns: Vec<f64> = closes.windows(2).map(|w| w[1].1 / w[0].1 - 1.0).collect();
        let vol = if returns.len() >= p.min_obs {
            ewma_vol(&returns, p.com_days, p.annualization)
        } else {
            None
        };
        let has_gap = window.iter().any(|d| !closes.iter().any(|(cd, _)| cd == d));
        match vol {
            None => {
                risk.ineligible
                    .insert(s.clone(), "insufficient_history".to_string());
            }
            Some(_) if has_gap => {
                risk.ineligible.insert(s.clone(), "gap".to_string());
            }
            Some(v) if !v.is_finite() => {
                risk.ineligible.insert(s.clone(), "non_finite".to_string());
            }
            Some(v) if v <= 0.0 => {
                risk.ineligible.insert(s.clone(), "zero_vol".to_string());
            }
            Some(v) => {
                risk.vols.insert(s.clone(), v);
                eligible_returns.push((s.clone(), returns));
            }
        }
    }
    for (x, rx) in &eligible_returns {
        for (y, ry) in &eligible_returns {
            if let Some(c) = ewma_cov(rx, ry, p.com_days, p.annualization) {
                risk.cov.insert((x.clone(), y.clone()), c);
            }
        }
    }
    risk
}
