//! The error type for `pt-live`.

use pt_alpaca::error::AlpacaError;
use pt_data::error::DataError;
use pt_engine::error::EngineError;
use pt_gate::error::GateError;
use pt_store::error::StoreError;
use thiserror::Error;

/// Errors raised by the paper-trading run cycle.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LiveError {
    #[error("another run is already in progress")]
    AlreadyRunning,
    #[error("unknown broker order status: {0}")]
    UnknownStatus(String),
    #[error("unexpected broker order status: {0}")]
    UnexpectedStatus(String),
    #[error("gate error: {0}")]
    Gate(#[from] GateError),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("alpaca error: {0}")]
    Alpaca(#[from] AlpacaError),
    #[error("data error: {0}")]
    Data(#[from] DataError),
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("invalid live input: {0}")]
    Invalid(String),
}
