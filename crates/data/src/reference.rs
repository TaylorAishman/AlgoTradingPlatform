//! Reference data sync: the trading calendar and assets.

use chrono::NaiveDate;
use pt_alpaca::http::HttpClient;
use pt_alpaca::reference::calendar;
use pt_store::db::Store;

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
