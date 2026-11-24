//! API credentials. Debug output never reveals the key id or the secret.

use std::fmt;

/// Alpaca API key pair.
#[derive(Clone)]
pub struct Credentials {
    key_id: String,
    secret_key: String,
}

impl Credentials {
    pub fn new(key_id: String, secret_key: String) -> Credentials {
        Credentials { key_id, secret_key }
    }

    /// Key id, for request headers.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Secret key, for request headers.
    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credentials(***)")
    }
}
