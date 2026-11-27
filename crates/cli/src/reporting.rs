//! The report inputs, assembled in one place for `run` and `report`.

use crate::strategies::StoredTsmom;
use anyhow::Context;
use chrono::Utc;
use pt_config::config::Config;
use pt_core::types::Symbol;
use pt_ops::report::{write_status, ReportInputs};
use pt_shadow::fidelity::fidelity;
use pt_store::db::Store;
use pt_strategy::tsmom::TsmomParams;
use std::path::PathBuf;

/// Write `status.md` and `status.json` and return their paths.
///
/// The fidelity section holds the ten summary values, in a fixed order.
pub fn write_report(
    store: &Store,
    cfg: &Config,
    stage: &str,
    validation_signoff: Option<&str>,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    let params = TsmomParams {
        lookback_months: cfg.strategy.lookback_months.clone(),
        risk_free_symbol: Symbol::new(&cfg.strategy.risk_free_symbol)
            .context("strategy.risk_free_symbol")?,
    };
    let strategy = StoredTsmom::new(store, params, cfg.data.start)?;
    let f = fidelity(store, cfg, &strategy)?;
    let rows: Vec<(String, String)> = [
        ("n_sessions", f.n_sessions.to_string()),
        ("n_rebalances", f.n_rebalances.to_string()),
        ("n_fills", f.n_fills.to_string()),
        ("replay_mismatches", f.replay_mismatches.to_string()),
        ("reconciliation_breaks", f.reconciliation_breaks.to_string()),
        ("missed_sessions", f.missed_sessions.to_string()),
        ("late_rebalances", f.late_rebalances.to_string()),
        ("mean_slippage_bps", f.mean_slippage_bps.to_string()),
        ("mean_diff_bps", f.mean_diff_bps.to_string()),
        ("max_abs_diff_bps", f.max_abs_diff_bps.to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let inputs = ReportInputs {
        store,
        cfg,
        stage,
        fidelity: &rows,
        validation_signoff,
        now: Utc::now(),
    };
    Ok(write_status(&inputs)?)
}
