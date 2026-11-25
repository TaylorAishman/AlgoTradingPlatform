//! Fidelity metrics: how closely paper trading follows its replay and its shadow.

use crate::error::ShadowError;
use crate::replay::replay_decision;
use pt_config::config::Config;
use pt_engine::decision::record_from_json;
use pt_store::db::Store;
use pt_store::trading::{Book, Mode};
use pt_strategy::strategy::Strategy;

/// Counts and means summarising paper trading against its shadow simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct Fidelity {
    /// Sessions with a stored shadow comparison.
    pub n_sessions: usize,
    /// Paper decisions (rebalances).
    pub n_rebalances: usize,
    /// Fills received from the broker.
    pub n_fills: usize,
    /// Paper decisions whose replay differs from what was stored.
    pub replay_mismatches: usize,
    /// Reconciliation and foreign-order halts, cleared or not.
    pub reconciliation_breaks: usize,
    /// Sessions whose paper snapshot was recorded after the UTC date of the
    /// next session. The last session is never counted.
    pub missed_sessions: usize,
    /// Paper decisions skipped as late, plus planned non-bootstrap decisions
    /// traded later than the session after their signal date.
    pub late_rebalances: usize,
    /// Mean slippage over recorded orders, in basis points.
    pub mean_slippage_bps: f64,
    /// Mean of paper return minus shadow return, in basis points.
    pub mean_diff_bps: f64,
    /// Largest absolute paper-minus-shadow return, in basis points.
    pub max_abs_diff_bps: f64,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Summarises the stored paper, shadow and slippage records. Each paper
/// decision is replayed with `replay_decision`, skipped_late ones included.
pub fn fidelity(
    store: &Store,
    cfg: &Config,
    strategy: &dyn Strategy,
) -> Result<Fidelity, ShadowError> {
    let days = store.shadow_days()?;
    let diffs: Vec<f64> = days.iter().map(|d| d.diff * 10_000.0).collect();
    let slippage: Vec<f64> = store.slippage_rows()?.iter().map(|r| r.bps).collect();

    let decisions = store.decisions(Mode::Paper)?;
    let mut replay_mismatches = 0;
    for row in &decisions {
        if !replay_decision(store, cfg, strategy, row.id)?.matches {
            replay_mismatches += 1;
        }
    }

    let calendar = store.calendar()?;
    let mut late_rebalances = 0;
    for row in &decisions {
        if row.status == "skipped_late" {
            late_rebalances += 1;
        } else if row.status == "planned" {
            let record = record_from_json(&row.inputs_json)?;
            let next = calendar.next_session(row.signal_date);
            if !record.bootstrap
                && matches!((row.trade_date, next), (Some(trade), Some(next)) if trade > next)
            {
                late_rebalances += 1;
            }
        }
    }

    let reconciliation_breaks = store
        .halts()?
        .iter()
        .filter(|h| h.reason.starts_with("reconciliation") || h.reason == "foreign_order")
        .count();

    let missed_sessions = store
        .equity_series(Book::Paper)?
        .iter()
        .filter(|s| {
            calendar
                .next_session(s.date)
                .is_some_and(|next| s.recorded_at.date_naive() > next)
        })
        .count();

    Ok(Fidelity {
        n_sessions: days.len(),
        n_rebalances: decisions.len(),
        n_fills: store.fills()?.len(),
        replay_mismatches,
        reconciliation_breaks,
        missed_sessions,
        late_rebalances,
        mean_slippage_bps: mean(&slippage),
        mean_diff_bps: mean(&diffs),
        max_abs_diff_bps: diffs.iter().fold(0.0, |m, d| f64::max(m, d.abs())),
    })
}
