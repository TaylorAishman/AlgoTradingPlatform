//! Data quality checks on bars.

use crate::error::DataError;
use chrono::NaiveDate;
use pt_alpaca::data::WireBar;
use pt_config::config::Config;
use pt_core::types::{Adjustment, Symbol};
use pt_store::db::Store;
use rust_decimal::Decimal;
use std::collections::HashSet;

/// R-124: a bar must be internally consistent. All prices are positive, the
/// high is at least every other price and the low is at most every other price.
pub fn check_bar(bar: &WireBar) -> Result<(), DataError> {
    let bad = |message: &str| DataError::BadBar {
        symbol: bar.symbol.clone(),
        date: bar.date,
        message: message.to_string(),
    };
    let prices = [bar.open, bar.high, bar.low, bar.close];
    if prices.iter().any(|p| *p <= Decimal::ZERO) {
        return Err(bad("prices must be positive"));
    }
    if prices.iter().any(|p| *p > bar.high) {
        return Err(bad("high is below another price"));
    }
    if prices.iter().any(|p| *p < bar.low) {
        return Err(bad("low is above another price"));
    }
    Ok(())
}

/// A trading session with no stored bar for a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub symbol: Symbol,
    pub date: NaiveDate,
}

/// R-126: reports, per symbol, every session up to `through` that has no
/// stored raw bar. Sessions before a symbol's first stored bar are not gaps.
/// The symbols are the universe plus the risk-free symbol.
pub fn check_gaps(store: &Store, cfg: &Config, through: NaiveDate) -> Result<Vec<Gap>, DataError> {
    let calendar = store.calendar()?;
    let mut names: Vec<&str> = cfg.universe.iter().map(|i| i.symbol.as_str()).collect();
    names.push(cfg.strategy.risk_free_symbol.as_str());
    let mut gaps = Vec::new();
    for name in names {
        let symbol = Symbol::new(name).map_err(|e| DataError::Invalid(format!("{name}: {e}")))?;
        let bars = store.bars_asof(
            &symbol,
            Adjustment::Raw,
            &cfg.data.feed,
            NaiveDate::MIN,
            through,
            None,
        )?;
        let Some(first) = bars.first() else {
            continue;
        };
        let have: HashSet<NaiveDate> = bars.iter().map(|b| b.date).collect();
        for session in calendar.sessions_between(first.date, through) {
            if !have.contains(session) {
                gaps.push(Gap {
                    symbol: symbol.clone(),
                    date: *session,
                });
            }
        }
    }
    Ok(gaps)
}
