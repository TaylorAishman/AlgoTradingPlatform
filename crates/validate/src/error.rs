//! The error type for `pt-validate`.

use pt_data::error::DataError;
use pt_engine::error::EngineError;
use pt_stats::error::StatsError;
use pt_store::error::StoreError;
use pt_vector::error::VectorError;
use thiserror::Error;

/// Errors raised while running, cross-checking and assessing backtests.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidateError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("data error: {0}")]
    Data(#[from] DataError),
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("vector error: {0}")]
    Vector(#[from] VectorError),
    #[error("stats error: {0}")]
    Stats(#[from] StatsError),
    #[error("results are not comparable: {0}")]
    NotComparable(String),
    #[error("too few trials")]
    TooFewTrials,
    #[error("invalid validation input: {0}")]
    Invalid(String),
}
