//! The error type for `pt-gate`.

use thiserror::Error;

/// Reasons the gate refuses an action, and owner-file errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GateError {
    #[error("the operating stage forbids this action")]
    StageForbids,
    #[error("the strategy is not allowed by the owner file")]
    StrategyNotAllowed,
    #[error("trading is halted")]
    Halted,
    #[error("the owner has not confirmed the account is flat")]
    FlatNotConfirmed,
    #[error("the order does not match the approved first order")]
    FirstOrderMismatch,
    #[error("the first order has already been sent")]
    FirstOrderAlreadySent,
    #[error("the owner file is inside the repository")]
    OwnerFileInRepo,
    #[error("owner file parse error: {0}")]
    Parse(String),
}
