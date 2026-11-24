//! The owner file: what the owner has approved. A missing file grants nothing.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::GateError;

/// Operating stage, ordered from least to most capable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Backtest,
    Ingest,
    DryRun,
    FirstOrder,
    Paper,
}

/// The owner's approval of one first order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstOrderApproval {
    pub symbol: String,
    pub side: String,
    pub max_qty: u64,
    pub valid_on: NaiveDate,
}

/// Everything the owner has granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerFile {
    pub stage: Stage,
    pub strategy_allowlist: Vec<String>,
    pub account_flat_confirmed_on: Option<NaiveDate>,
    pub first_order: Option<FirstOrderApproval>,
    pub validation_signoff: Option<String>,
    pub alert_webhook_url: Option<String>,
}

impl Default for OwnerFile {
    /// Backtest stage, nothing granted.
    fn default() -> Self {
        OwnerFile {
            stage: Stage::Backtest,
            strategy_allowlist: Vec::new(),
            account_flat_confirmed_on: None,
            first_order: None,
            validation_signoff: None,
            alert_webhook_url: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFirstOrder {
    symbol: String,
    side: String,
    max_qty: u64,
    valid_on: toml::value::Datetime,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOwner {
    stage: Stage,
    #[serde(default)]
    strategy_allowlist: Vec<String>,
    account_flat_confirmed_on: Option<toml::value::Datetime>,
    first_order: Option<RawFirstOrder>,
    validation_signoff: Option<String>,
    alert_webhook_url: Option<String>,
}

/// Convert a TOML local date (no time, no offset) to a `NaiveDate`.
fn to_date(value: toml::value::Datetime, key: &str) -> Result<NaiveDate, GateError> {
    let bad = || GateError::Parse(format!("{key} must be a date like 2026-12-01"));
    if value.time.is_some() || value.offset.is_some() {
        return Err(bad());
    }
    let d = value.date.ok_or_else(bad)?;
    NaiveDate::from_ymd_opt(i32::from(d.year), u32::from(d.month), u32::from(d.day)).ok_or_else(bad)
}

/// Canonicalize `path`; if it does not exist, canonicalize its nearest existing
/// ancestor and re-append the missing tail.
fn resolve(path: &Path) -> PathBuf {
    let mut tail = Vec::new();
    let mut cur = path;
    loop {
        if let Ok(base) = cur.canonicalize() {
            return tail.iter().rev().fold(base, |acc, c| acc.join(c));
        }
        match (cur.parent(), cur.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name.to_owned());
                cur = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Load the owner file. A path inside `repo_root` is refused (R-262).
/// A missing file is the default (backtest, nothing granted).
pub fn load_owner_file(path: &Path, repo_root: &Path) -> Result<OwnerFile, GateError> {
    if resolve(path).starts_with(resolve(repo_root)) {
        return Err(GateError::OwnerFileInRepo);
    }
    if !path.exists() {
        return Ok(OwnerFile::default());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| GateError::Parse(format!("cannot read owner file: {e}")))?;
    let raw: RawOwner = toml::from_str(&text).map_err(|e| GateError::Parse(e.to_string()))?;
    let first_order = match raw.first_order {
        Some(f) => Some(FirstOrderApproval {
            symbol: f.symbol,
            side: f.side,
            max_qty: f.max_qty,
            valid_on: to_date(f.valid_on, "first_order.valid_on")?,
        }),
        None => None,
    };
    let account_flat_confirmed_on = match raw.account_flat_confirmed_on {
        Some(d) => Some(to_date(d, "account_flat_confirmed_on")?),
        None => None,
    };
    Ok(OwnerFile {
        stage: raw.stage,
        strategy_allowlist: raw.strategy_allowlist,
        account_flat_confirmed_on,
        first_order,
        validation_signoff: raw.validation_signoff,
        alert_webhook_url: raw.alert_webhook_url,
    })
}
