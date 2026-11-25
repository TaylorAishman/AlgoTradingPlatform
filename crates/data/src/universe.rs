//! Universe verification: each candidate is checked against the broker's asset status.

use chrono::NaiveDate;
use pt_alpaca::http::HttpClient;
use pt_alpaca::reference::asset;
use pt_config::config::Config;
use pt_core::types::Symbol;
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
/// `not_shortable`, `hard_to_borrow`. Returns the checks in universe order.
pub fn verify_universe(
    _store: &Store,
    http: &HttpClient,
    cfg: &Config,
    _through: NaiveDate,
) -> Result<Vec<UniverseCheck>, DataError> {
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
        out.push(UniverseCheck {
            symbol,
            pass: reasons.is_empty(),
            reasons,
        });
    }
    Ok(out)
}
