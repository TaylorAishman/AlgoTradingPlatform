//! The error type for `pt-engine`.

use chrono::NaiveDate;
use pt_core::error::CoreError;
use pt_portfolio::error::PortfolioError;
use pt_strategy::error::StrategyError;
use thiserror::Error;

/// Errors raised by the decision function, the simulated broker and the backtest loop.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    #[error("no bar for {symbol} on {date}")]
    MissingBar { symbol: String, date: NaiveDate },
    #[error("data for {symbol} is not ready on {date}")]
    DataNotReady { symbol: String, date: NaiveDate },
    #[error("missing inputs for the impact cost model: {0}")]
    MissingImpactInputs(String),
    #[error("strategy error: {0}")]
    Strategy(#[from] StrategyError),
    #[error("portfolio error: {0}")]
    Portfolio(#[from] PortfolioError),
    #[error("core error: {0}")]
    Core(#[from] CoreError),
    #[error("invalid engine input: {0}")]
    Invalid(String),
}
