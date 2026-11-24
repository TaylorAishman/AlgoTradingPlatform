//! The error type for `pt-shadow`.

use pt_data::error::DataError;
use pt_engine::error::EngineError;
use pt_store::error::StoreError;
use thiserror::Error;

/// Errors raised by the shadow simulation and its sign-off evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ShadowError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("data error: {0}")]
    Data(#[from] DataError),
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid shadow input: {0}")]
    Invalid(String),
}
