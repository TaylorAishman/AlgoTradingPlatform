//! Decision replay: recompute a stored decision from its stored inputs and the
//! bars as they were when it was made, and compare it with what was stored.

use crate::error::ShadowError;
use pt_config::config::Config;
use pt_core::types::{PriceBasis, Symbol};
use pt_data::view::StoreView;
use pt_engine::decision::{decide, decision_to_json, params_from_config, record_from_json};
use pt_store::db::Store;
use pt_store::trading::{DecisionRow, Mode};
use pt_strategy::strategy::Strategy;
use std::collections::BTreeSet;

/// The outcome of replaying one stored decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayResult {
    pub decision_id: i64,
    pub matches: bool,
    pub diffs: Vec<String>,
}

fn find_decision(store: &Store, decision_id: i64) -> Result<DecisionRow, ShadowError> {
    for mode in [Mode::Paper, Mode::DryRun] {
        if let Some(row) = store
            .decisions(mode)?
            .into_iter()
            .find(|r| r.id == decision_id)
        {
            return Ok(row);
        }
    }
    Err(ShadowError::NotFound(format!("decision {decision_id}")))
}

/// Recomputes decision `decision_id` from its stored record, over the bars of
/// its feed as of its `data_asof`, and compares the result with its stored
/// outputs. `matches` is true exactly when `diffs` is empty.
pub fn replay_decision(
    store: &Store,
    cfg: &Config,
    strategy: &dyn Strategy,
    decision_id: i64,
) -> Result<ReplayResult, ShadowError> {
    let row = find_decision(store, decision_id)?;
    let record = record_from_json(&row.inputs_json)?;

    let mut symbols: BTreeSet<Symbol> = record.inputs.universe.iter().cloned().collect();
    symbols.extend(record.inputs.positions.keys().cloned());
    symbols.insert(Symbol::new(&cfg.strategy.risk_free_symbol).map_err(|e| {
        ShadowError::Invalid(format!(
            "risk-free symbol \"{}\": {e}",
            cfg.strategy.risk_free_symbol
        ))
    })?);
    let symbols: Vec<Symbol> = symbols.into_iter().collect();

    let view = StoreView::load(
        store,
        &symbols,
        &record.feed,
        PriceBasis::Raw,
        Some(record.data_asof),
    )?;
    let decision = decide(&record.inputs, strategy, &view, &params_from_config(cfg))?;
    let replayed = decision_to_json(&decision);

    let mut diffs = Vec::new();
    if replayed != row.outputs_json {
        diffs.push("outputs".to_string());
    }
    Ok(ReplayResult {
        decision_id,
        matches: diffs.is_empty(),
        diffs,
    })
}
