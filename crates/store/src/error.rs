//! The error type for `pt-store`.

use thiserror::Error;

/// Errors raised by the SQLite store.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StoreError {
    #[error("sql error: {0}")]
    Sql(String),
    #[error("duplicate record: {0}")]
    Duplicate(String),
    #[error("illegal transition from {from} to {to}")]
    IllegalTransition { from: String, to: String },
    #[error("database schema version {found} is newer than supported version {supported}")]
    SchemaTooNew { found: i64, supported: i64 },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("codec error: {0}")]
    Codec(String),
    #[error("invalid input: {0}")]
    Invalid(String),
}
