//! The error type for `pt-ops`.

use pt_store::error::StoreError;
use thiserror::Error;

/// Errors raised while alerting, logging and reporting.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpsError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("io error: {0}")]
    Io(String),
}
