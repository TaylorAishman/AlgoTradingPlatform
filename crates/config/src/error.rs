//! The error type for `pt-config`.

use thiserror::Error;

/// Errors raised while loading or validating configuration and credentials.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("i/o error: {0}")]
    Io(String),
    #[error("config parse error: {0}")]
    Parse(String),
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error("API credentials are missing")]
    MissingCredentials,
    #[error("secrets file has insecure permissions: {0}")]
    InsecureSecretsFile(String),
}
