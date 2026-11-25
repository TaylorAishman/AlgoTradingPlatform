//! Reference data sync: the trading calendar and assets.

use chrono::{DateTime, NaiveDate, Utc};
use pt_alpaca::http::HttpClient;
use pt_alpaca::reference::{asset, calendar};
use pt_config::config::Config;
use pt_core::types::Symbol;
use pt_store::db::Store;
use pt_store::market::AssetSnapshot;

use crate::error::DataError;

/// Fetches the sessions between `start` and `end` from Alpaca and stores
/// them; returns how many sessions were fetched.
pub fn sync_calendar(
    store: &Store,
    http: &HttpClient,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<usize, DataError> {
    let sessions = calendar(http, start, end)?;
    store.put_sessions(&sessions)?;
    Ok(sessions.len())
}

/// Fetches and stores an asset snapshot for every universe symbol (the
/// risk-free symbol is skipped). A symbol the broker does not know gets a
/// snapshot with all flags false. Returns the snapshots in universe order.
pub fn sync_assets(
    store: &Store,
    http: &HttpClient,
    config: &Config,
    fetched_at: DateTime<Utc>,
) -> Result<Vec<AssetSnapshot>, DataError> {
    let mut out = Vec::new();
    for inst in &config.universe {
        if inst.symbol == config.strategy.risk_free_symbol {
            continue;
        }
        let symbol = Symbol::new(&inst.symbol)
            .map_err(|e| DataError::Invalid(format!("universe symbol {}: {e}", inst.symbol)))?;
        let snapshot = match asset(http, &inst.symbol)? {
            Some(a) => AssetSnapshot {
                symbol,
                tradable: a.tradable,
                shortable: a.shortable,
                easy_to_borrow: a.easy_to_borrow,
            },
            None => AssetSnapshot {
                symbol,
                tradable: false,
                shortable: false,
                easy_to_borrow: false,
            },
        };
        store.put_asset_snapshot(&snapshot, fetched_at)?;
        out.push(snapshot);
    }
    Ok(out)
}
