//! The error type for `pt-core`.

use thiserror::Error;

/// Errors raised while building or validating core domain values.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("invalid symbol: {0}")]
    InvalidSymbol(String),
    #[error("price must be positive")]
    NonPositivePrice,
    #[error("value is not finite")]
    NotFinite,
    #[error("invalid order: {0}")]
    InvalidOrder(String),
    #[error("invalid cost specification: {0}")]
    InvalidCost(String),
}
