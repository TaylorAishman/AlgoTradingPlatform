//! Deterministic client order identifiers.
//!
//! The same inputs always give the same identifier, so a retried submission
//! is recognised by the broker as a duplicate. Alpaca allows at most 128
//! characters.

use chrono::NaiveDate;

/// Identifier of a rebalance order:
/// `pt-<strategy id>-<signal date yyyymmdd>-<symbol>-<leg>`.
pub fn client_order_id(strategy_id: &str, signal_date: NaiveDate, symbol: &str, leg: u8) -> String {
    format!(
        "pt-{strategy_id}-{}-{symbol}-{leg}",
        signal_date.format("%Y%m%d")
    )
}

/// Identifier of the approved first order: `pt-first-<valid_on yyyymmdd>-<symbol>`.
pub fn first_order_id(valid_on: NaiveDate, symbol: &str) -> String {
    format!("pt-first-{}-{symbol}", valid_on.format("%Y%m%d"))
}
