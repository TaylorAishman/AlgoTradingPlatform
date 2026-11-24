//! The error type for `pt-alpaca`.

use thiserror::Error;

/// Errors raised by the Alpaca REST client.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AlpacaError {
    #[error("endpoint not allowed: {0}")]
    EndpointNotAllowed(String),
    #[error("rate limited")]
    RateLimited,
    #[error("server error: HTTP {status}")]
    Server { status: u16 },
    #[error("request rejected with HTTP {status}: {message}")]
    Rejected { status: u16, message: String },
    #[error("feed {feed} is not permitted: {message}")]
    FeedNotPermitted { feed: String, message: String },
    #[error("the order permit does not match the order")]
    PermitMismatch,
    #[error("unexpected bar time: {0}")]
    UnexpectedBarTime(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("transport error: {0}")]
    Transport(String),
}
