//! Overfitting measures over the trial registry: the Deflated Sharpe Ratio of a stored run
//! and the Probability of Backtest Overfitting of a set of stored runs.

use std::collections::{BTreeMap, BTreeSet};

use pt_config::config::Config;
use pt_stats::descriptive::{kurtosis, returns_from_equity, skewness, std_sample};
use pt_stats::dsr::{deflated_sharpe, expected_max_sharpe};
use pt_stats::ols::{ols, Ols};
use pt_stats::pbo::{pbo_cscv, PboResult};
use pt_stats::perf::{annualized_sharpe, sharpe};
use pt_store::db::Store;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

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

/// The Probability of Backtest Overfitting of stored runs, on the daily returns of their
/// equity over the dates all of them share. Columns follow `run_ids`; `s` is the partition count.
pub fn pbo_for_runs(store: &Store, run_ids: &[i64], s: usize) -> Result<PboResult, ValidateError> {
    if run_ids.len() < 2 {
        return Err(ValidateError::TooFewTrials);
    }
    let mut curves: Vec<BTreeMap<chrono::NaiveDate, f64>> = Vec::with_capacity(run_ids.len());
    for id in run_ids {
        let mut curve = BTreeMap::new();
        for (d, e) in store.backtest_equity(*id)? {
            let v = e
                .to_f64()
                .ok_or_else(|| ValidateError::Invalid(format!("run {id} equity is not finite")))?;
            curve.insert(d, v);
        }
        curves.push(curve);
    }
    let mut shared: BTreeSet<chrono::NaiveDate> = curves[0].keys().copied().collect();
    for c in &curves[1..] {
        shared.retain(|d| c.contains_key(d));
    }
    let columns: Vec<Vec<f64>> = curves
        .iter()
        .map(|c| {
            let eq: Vec<f64> = shared.iter().map(|d| c[d]).collect();
            returns_from_equity(&eq)
        })
        .collect();
    let rows = columns[0].len();
    let matrix: Vec<Vec<f64>> = (0..rows)
        .map(|r| columns.iter().map(|c| c[r]).collect())
        .collect();
    Ok(pbo_cscv(&matrix, s)?)
}

/// A strategy run set against a benchmark run over the dates they share.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkReport {
    pub strategy_sharpe: f64,
    pub benchmark_sharpe: f64,
    /// Regression of the strategy's daily returns on the benchmark's.
    pub ols: Ols,
}

/// Compare stored run `run_id` with `benchmark_run_id` on the dates both have equity for.
/// Each Sharpe is annualised on the run's daily returns minus the risk-free symbol's,
/// aligned by date; the regression is on the plain daily returns.
pub fn compare_to_benchmark(
    store: &Store,
    cfg: &Config,
    run_id: i64,
    benchmark_run_id: i64,
    periods_per_year: f64,
) -> Result<BenchmarkReport, ValidateError> {
    let strat = store.backtest_equity(run_id)?;
    let bench = store.backtest_equity(benchmark_run_id)?;
    let in_bench: BTreeSet<chrono::NaiveDate> = bench.iter().map(|(d, _)| *d).collect();
    let shared: BTreeSet<chrono::NaiveDate> = strat
        .iter()
        .map(|(d, _)| *d)
        .filter(|d| in_bench.contains(d))
        .collect();
    let (first, last) = match (shared.first(), shared.last()) {
        (Some(f), Some(l)) => (*f, *l),
        _ => {
            return Err(ValidateError::Invalid(format!(
                "runs {run_id} and {benchmark_run_id} share no dates"
            )))
        }
    };
    let restrict = |eq: Vec<(chrono::NaiveDate, Decimal)>| -> Vec<(chrono::NaiveDate, Decimal)> {
        let mut v: Vec<_> = eq.into_iter().filter(|(d, _)| shared.contains(d)).collect();
        v.sort_by_key(|(d, _)| *d);
        v
    };
    let (strat, bench) = (restrict(strat), restrict(bench));
    let closes = risk_free_closes(store, cfg, first, last)?;
    let rf = &cfg.strategy.risk_free_symbol;
    let strategy_sharpe =
        annualized_sharpe(&excess_returns(&strat, rf, &closes)?, periods_per_year);
    let benchmark_sharpe =
        annualized_sharpe(&excess_returns(&bench, rf, &closes)?, periods_per_year);
    let plain = |eq: &[(chrono::NaiveDate, Decimal)]| -> Result<Vec<f64>, ValidateError> {
        let v = eq
            .iter()
            .map(|(_, e)| {
                e.to_f64()
                    .ok_or_else(|| ValidateError::Invalid(format!("{e} does not fit in f64")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(returns_from_equity(&v))
    };
    let ols = ols(&plain(&strat)?, &plain(&bench)?)?;
    Ok(BenchmarkReport {
        strategy_sharpe,
        benchmark_sharpe,
        ols,
    })
}
