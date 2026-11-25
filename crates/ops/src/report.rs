//! The status report the owner reads.

use crate::error::OpsError;
use chrono::{DateTime, Utc};
use pt_config::config::Config;
use pt_core::types::Adjustment;
use pt_store::db::Store;
use pt_store::trading::Book;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{json, Value};
use std::path::PathBuf;

/// Everything the report needs.
pub struct ReportInputs<'a> {
    pub store: &'a Store,
    pub cfg: &'a Config,
    pub stage: &'a str,
    /// Label and value pairs; empty if none.
    pub fidelity: &'a [(String, String)],
    /// From the owner file.
    pub validation_signoff: Option<&'a str>,
    pub now: DateTime<Utc>,
}

/// Writes `status.json` into the configured output directory and returns the
/// paths of `status.md` and `status.json`.
pub fn write_status(inp: &ReportInputs) -> Result<(PathBuf, PathBuf), OpsError> {
    let dir = PathBuf::from(&inp.cfg.run.output_dir);
    std::fs::create_dir_all(&dir).map_err(|e| OpsError::Io(e.to_string()))?;
    let json_path = dir.join("status.json");
    let body = serde_json::to_string_pretty(&status_json(inp)?)
        .map_err(|e| OpsError::Io(e.to_string()))?;
    std::fs::write(&json_path, body).map_err(|e| OpsError::Io(e.to_string()))?;
    Ok((dir.join("status.md"), json_path))
}

fn status_json(inp: &ReportInputs) -> Result<Value, OpsError> {
    let series = inp.store.equity_series(Book::Paper)?;
    let latest = series.last();
    let feed = inp.cfg.data.feed.as_str();
    let mut positions = Vec::new();
    if let Some(snap) = latest {
        for (symbol, qty) in inp.store.positions_at(Book::Paper, snap.date)? {
            let bars = inp.store.bars_asof(
                &symbol,
                Adjustment::Raw,
                feed,
                chrono::NaiveDate::MIN,
                snap.date,
                None,
            )?;
            let value = bars.last().map(|b| b.close.notional(qty).0);
            positions.push(json!({
                "symbol": symbol.as_str(),
                "qty": qty,
                "value": value.map(|v| v.to_string()),
            }));
        }
    }
    let gross = latest.and_then(|s| {
        s.positions_value
            .abs()
            .checked_div(s.equity)
            .and_then(|g| g.to_f64())
    });
    Ok(json!({
        "generated_at": inp.now.to_rfc3339(),
        "stage": inp.stage,
        "feed": feed,
        "basis": "price_return",
        "last_run": Value::Null,
        "equity": latest.map(|s| s.equity.to_string()),
        "cash": latest.map(|s| s.cash.to_string()),
        "gross_exposure": gross,
        "pnl": Value::Null,
        "positions": positions,
        "last_rebalance": Value::Null,
        "active_halts": Vec::<Value>::new(),
        "trial_count": Value::Null,
        "fidelity": inp.fidelity.iter().map(|(k, v)| json!({"label": k, "value": v})).collect::<Vec<_>>(),
        "validation_signoff": inp.validation_signoff,
        "disclaimer": Value::Null,
    }))
}
