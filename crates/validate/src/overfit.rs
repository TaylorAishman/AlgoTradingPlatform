//! Overfitting measures over the trial registry: the Deflated Sharpe Ratio of a stored run.

use pt_config::config::Config;
use pt_stats::descriptive::{kurtosis, skewness, std_sample};
use pt_stats::dsr::{deflated_sharpe, expected_max_sharpe};
use pt_stats::perf::sharpe;
use pt_store::db::Store;

use crate::error::ValidateError;
use crate::runner::{excess_returns, risk_free_closes};

/// The Deflated Sharpe Ratio of a run, with the inputs it was computed from.
#[derive(Debug, Clone, PartialEq)]
pub struct DsrReport {
    pub dsr: f64,
    pub sr: f64,
    pub sr0: f64,
    pub n_trials: u32,
    pub skew: f64,
    pub kurt: f64,
    pub t: usize,
}

/// The Deflated Sharpe Ratio of stored run `run_id`, on its daily excess returns over the
/// config's risk-free symbol, deflated by the trials registered under `family`.
pub fn dsr_for_run(
    store: &Store,
    cfg: &Config,
    run_id: i64,
    family: &str,
) -> Result<DsrReport, ValidateError> {
    let equity = store.backtest_equity(run_id)?;
    let (first, last) = match (equity.first(), equity.last()) {
        (Some(f), Some(l)) => (f.0, l.0),
        _ => {
            return Err(ValidateError::Invalid(format!(
                "run {run_id} has no equity"
            )))
        }
    };
    let closes = risk_free_closes(store, cfg, first, last)?;
    let excess = excess_returns(&equity, &cfg.strategy.risk_free_symbol, &closes)?;

    let trials = store.trials(family)?;
    let n_trials = u32::try_from(trials.len())
        .map_err(|_| ValidateError::Invalid("too many trials".into()))?;
    let srs: Vec<f64> = trials.iter().map(|t| t.sharpe).collect();
    let var_sr = if srs.len() > 1 {
        std_sample(&srs).powi(2)
    } else {
        0.0
    };

    let sr = sharpe(&excess);
    let (skew, kurt) = (skewness(&excess), kurtosis(&excess));
    let t = excess.len();
    let dsr = deflated_sharpe(sr, t, skew, kurt, var_sr, n_trials)?;
    Ok(DsrReport {
        dsr,
        sr,
        sr0: expected_max_sharpe(var_sr, n_trials),
        n_trials,
        skew,
        kurt,
        t,
    })
}
