//! The error type for `pt-strategy`.

use chrono::NaiveDate;
use thiserror::Error;

/// Errors raised while estimating risk or computing signals.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrategyError {
    #[error("no risk-free rate for {0}")]
    MissingRiskFree(NaiveDate),
    #[error("{0} is not a trading session")]
    NotASession(NaiveDate),
    #[error("invalid strategy input: {0}")]
    Invalid(String),
}
