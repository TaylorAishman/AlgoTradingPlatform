//! The error type for `pt-portfolio`.

use thiserror::Error;

/// Errors raised while building target weights or the order plan.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PortfolioError {
    #[error("no price for {0}")]
    MissingPrice(String),
    #[error("invalid portfolio input: {0}")]
    Invalid(String),
}
