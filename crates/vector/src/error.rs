//! The error type for `pt-vector`.

use chrono::NaiveDate;
use thiserror::Error;

/// Errors raised by the vectorised calculator.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VectorError {
    #[error("inputs have the wrong shape: {0}")]
    Shape(String),
    #[error("price must be positive")]
    NonPositivePrice,
    #[error("trade date {0} is not in the price index")]
    UnknownTradeDate(NaiveDate),
    #[error("the impact cost model is not supported")]
    ImpactNotSupported,
    #[error("input lengths differ")]
    LengthMismatch,
}
