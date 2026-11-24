//! The error type for `pt-stats`.

use thiserror::Error;

/// Errors raised by the statistics functions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StatsError {
    #[error("argument out of domain: {0}")]
    OutOfDomain(String),
    #[error("too few observations")]
    TooFewObservations,
    #[error("invalid partition: {0}")]
    InvalidPartition(String),
}
