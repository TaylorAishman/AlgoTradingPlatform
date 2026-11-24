//! The error type for `pt-data`.

use chrono::NaiveDate;
use pt_alpaca::error::AlpacaError;
use pt_store::error::StoreError;
use thiserror::Error;

/// Errors raised while ingesting, importing or checking market data.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DataError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("alpaca error: {0}")]
    Alpaca(#[from] AlpacaError),
    #[error("csv error at line {line}: {message}")]
    Csv { line: usize, message: String },
    #[error("bad bar for {symbol} on {date}: {message}")]
    BadBar {
        symbol: String,
        date: NaiveDate,
        message: String,
    },
    #[error("invalid data: {0}")]
    Invalid(String),
}
