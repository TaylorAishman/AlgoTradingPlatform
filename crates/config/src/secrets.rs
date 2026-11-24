//! API key types whose `Debug` and `Display` never print the value.

use std::fmt;

/// A secret string. Printing it shows `***`; use [`Secret::expose`] to read it.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// The underlying value, for deliberate use only.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// Alpaca API credentials.
#[derive(Debug, Clone)]
pub struct ApiKeys {
    pub key_id: Secret,
    pub secret_key: Secret,
}
