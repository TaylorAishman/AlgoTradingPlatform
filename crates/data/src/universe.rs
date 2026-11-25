//! Universe verification: each candidate is checked against the broker's asset status.

use std::collections::HashSet;

use chrono::NaiveDate;
use pt_alpaca::http::HttpClient;
use pt_alpaca::reference::asset;
use pt_config::config::Config;
use pt_core::types::{Adjustment, Symbol};
use pt_store::db::Store;

use crate::error::DataError;

/// The result of checking one universe symbol; `reasons` is empty when `pass` is true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseCheck {
    pub symbol: Symbol,
    pub pass: bool,
    pub reasons: Vec<String>,
}

/// Checks every universe symbol (the risk-free symbol is skipped) through
/// `through`. Asset-status reasons: `not_found`, `not_tradable`,
/// `not_shortable`, `hard_to_borrow`. History reasons (R-134):
/// `history_starts_late` (first raw bar after the first session on or after
/// `data.start`, or no bars) and `gaps:<n>` (sessions missing after the first
/// bar). Returns the checks in universe order.
pub fn verify_universe(
    store: &Store,
    http: &HttpClient,
    cfg: &Config,
    through: NaiveDate,
) -> Result<Vec<UniverseCheck>, DataError> {
    let calendar = store.calendar()?;
    let mut out = Vec::new();
    for inst in &cfg.universe {
        if inst.symbol == cfg.strategy.risk_free_symbol {
            continue;
        }
        let symbol = Symbol::new(&inst.symbol)
            .map_err(|e| DataError::Invalid(format!("universe symbol {}: {e}", inst.symbol)))?;
        let mut reasons = Vec::new();
        match asset(http, &inst.symbol)? {
            None => reasons.push("not_found".to_string()),
            Some(a) => {
                if !a.tradable {
                    reasons.push("not_tradable".to_string());
                }
                if !a.shortable {
                    reasons.push("not_shortable".to_string());
                } else if !a.easy_to_borrow {
                    reasons.push("hard_to_borrow".to_string());
                }
            }
        }
        let bars = store.bars_asof(
            &symbol,
            Adjustment::Raw,
            &cfg.data.feed,
            NaiveDate::MIN,
            through,
            None,
        )?;
        match bars.first() {
            None => reasons.push("history_starts_late".to_string()),
            Some(first) => {
                let expected = calendar
                    .sessions_between(cfg.data.start, through)
                    .first()
                    .copied();
                if expected.is_some_and(|e| first.date > e) {
                    reasons.push("history_starts_late".to_string());
                }
                let have: HashSet<NaiveDate> = bars.iter().map(|b| b.date).collect();
                let missing = calendar
                    .sessions_between(first.date, through)
                    .iter()
                    .filter(|d| !have.contains(*d))
                    .count();
                if missing > 0 {
                    reasons.push(format!("gaps:{missing}"));
                }
            }
        }
        out.push(UniverseCheck {
            symbol,
            pass: reasons.is_empty(),
            reasons,
        });
    }
    Ok(out)
}
