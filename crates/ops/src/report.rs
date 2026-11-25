//! The status report the owner reads.

use crate::error::OpsError;
use chrono::{DateTime, Utc};
use pt_config::config::Config;
use pt_core::types::Adjustment;
use pt_store::db::Store;
use pt_store::trading::{Book, Mode};
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

/// Writes `status.json` and `status.md` into the configured output directory and returns the
/// paths of `status.md` and `status.json`.
pub fn write_status(inp: &ReportInputs) -> Result<(PathBuf, PathBuf), OpsError> {
    let dir = PathBuf::from(&inp.cfg.run.output_dir);
    std::fs::create_dir_all(&dir).map_err(|e| OpsError::Io(e.to_string()))?;
    let json_path = dir.join("status.json");
    let md_path = dir.join("status.md");
    let status = status_json(inp)?;
    let body = serde_json::to_string_pretty(&status).map_err(|e| OpsError::Io(e.to_string()))?;
    replace_file(&json_path, body.as_ref())?;
    replace_file(&md_path, status_md(&status).as_bytes())?;
    Ok((md_path, json_path))
}

/// Hours from the finish of the latest `ok` run to `now`; `None` if no run has finished ok.
pub fn last_ok_run_age_hours(store: &Store, now: DateTime<Utc>) -> Result<Option<f64>, OpsError> {
    let finished = store
        .runs()?
        .into_iter()
        .filter(|r| r.status == "ok")
        .filter_map(|r| r.finished_at)
        .max();
    Ok(finished.map(|t| (now - t).num_seconds() as f64 / 3600.0))
}

/// Write `bytes` to a temporary file beside `path`, then rename it over `path`.
fn replace_file(path: &std::path::Path, bytes: &[u8]) -> Result<(), OpsError> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let io = |e: std::io::Error| OpsError::Io(e.to_string());
    std::fs::write(&tmp, bytes).map_err(io)?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        io(e)
    })
}

fn text(v: &Value) -> String {
    match v {
        Value::Null => "-".to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn status_md(s: &Value) -> String {
    let mut out = String::new();
    out.push_str("# Status\n\n");
    out.push_str("Paper trading. Not investment advice.\n\n");
    out.push_str("Returns are price returns: the paper account does not simulate dividends.\n\n");
    out.push_str(&format!(
        "- Generated at: {}\n- Stage: {}\n- Feed: {}\n- Equity: {}\n- Cash: {}\n\n",
        text(&s["generated_at"]),
        text(&s["stage"]),
        text(&s["feed"]),
        text(&s["equity"]),
        text(&s["cash"]),
    ));
    out.push_str("## Positions\n\n| Symbol | Qty | Value |\n| --- | --- | --- |\n");
    for p in s["positions"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            text(&p["symbol"]),
            text(&p["qty"]),
            text(&p["value"])
        ));
    }
    out.push_str("\n## Active halts\n\n");
    let halts = s["active_halts"].as_array().cloned().unwrap_or_default();
    if halts.is_empty() {
        out.push_str("None.\n");
    }
    for h in &halts {
        out.push_str(&format!(
            "- {}: {}\n",
            text(&h["reason"]),
            text(&h["detail"])
        ));
    }
    out
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
    let last_run = inp.store.runs()?.pop().map(|r| {
        json!({
            "id": r.id,
            "stage": r.stage,
            "status": r.status,
            "started_at": r.started_at.to_rfc3339(),
            "finished_at": r.finished_at.map(|t| t.to_rfc3339()),
        })
    });
    let snaps: Vec<_> = series.iter().map(|s| (s.date, s.equity)).collect();
    let pnl = latest
        .and_then(|s| crate::pnl::pnl(&snaps, s.date))
        .map(|p| {
            json!({
                "day": p.day.to_string(),
                "mtd": p.mtd.to_string(),
                "ytd": p.ytd.to_string(),
                "inception": p.inception.to_string(),
            })
        });
    let halts: Vec<Value> = inp
        .store
        .active_halts()?
        .into_iter()
        .map(|h| json!({"reason": h.reason, "detail": h.detail, "set_at": h.set_at.to_rfc3339()}))
        .collect();
    let last_rebalance = inp
        .store
        .decisions(Mode::Paper)?
        .into_iter()
        .max_by_key(|d| (d.signal_date, d.id))
        .map(|d| {
            json!({
                "signal_date": d.signal_date.to_string(),
                "trade_date": d.trade_date.map(|t| t.to_string()),
                "status": d.status,
                "created_at": d.created_at.to_rfc3339(),
            })
        });
    let trial_count = inp.store.trials(&inp.cfg.strategy.id)?.len();
    Ok(json!({
        "generated_at": inp.now.to_rfc3339(),
        "stage": inp.stage,
        "feed": feed,
        "basis": "price_return",
        "last_run": last_run,
        "equity": latest.map(|s| s.equity.to_string()),
        "cash": latest.map(|s| s.cash.to_string()),
        "gross_exposure": gross,
        "pnl": pnl,
        "positions": positions,
        "last_rebalance": last_rebalance,
        "active_halts": halts,
        "trial_count": trial_count,
        "fidelity": inp.fidelity.iter().map(|(k, v)| json!({"label": k, "value": v})).collect::<Vec<_>>(),
        "validation_signoff": inp.validation_signoff,
        "disclaimer": "Paper trading. Not investment advice.",
    }))
}
