//! Risk estimates from adjusted closes (Arch 5.10.1).

use std::collections::BTreeMap;

use chrono::NaiveDate;
pub use pt_core::signal::RiskEstimates;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;

use crate::vol::ewma_vol;

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
/// `min_obs` returns is ineligible with reason `insufficient_history`.
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
    for s in symbols {
        let closes = view.adj_closes(s, asof, p.max_obs + 1);
        let returns: Vec<f64> = closes.windows(2).map(|w| w[1].1 / w[0].1 - 1.0).collect();
        let vol = if returns.len() >= p.min_obs {
            ewma_vol(&returns, p.com_days, p.annualization)
        } else {
            None
        };
        match vol {
            Some(v) => {
                risk.vols.insert(s.clone(), v);
            }
            None => {
                risk.ineligible
                    .insert(s.clone(), "insufficient_history".to_string());
            }
        }
    }
    risk
}
