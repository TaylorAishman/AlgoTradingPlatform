//! The owner file: what the owner has approved. A missing file grants nothing.

use std::path::Path;

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

/// Load the owner file. A missing file is the default (backtest, nothing granted).
pub fn load_owner_file(path: &Path, _repo_root: &Path) -> Result<OwnerFile, GateError> {
    if !path.exists() {
        return Ok(OwnerFile::default());
    }
    Err(GateError::Parse(
        "owner file parsing is not implemented".into(),
    ))
}
