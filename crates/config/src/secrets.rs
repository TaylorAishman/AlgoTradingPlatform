//! API key types whose `Debug` and `Display` never print the value.

use std::collections::BTreeMap;
use std::fmt;

use crate::error::ConfigError;

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

/// Load API keys from an environment map (`APCA_API_KEY_ID`, `APCA_API_SECRET_KEY`).
///
/// Both variables must be present and non-empty, otherwise
/// [`ConfigError::MissingCredentials`] is returned.
pub fn load_api_keys(env: &BTreeMap<String, String>) -> Result<ApiKeys, ConfigError> {
    let get = |name: &str| env.get(name).filter(|v| !v.is_empty());
    if let (Some(id), Some(secret)) = (get("APCA_API_KEY_ID"), get("APCA_API_SECRET_KEY")) {
        return Ok(ApiKeys {
            key_id: Secret::new(id.clone()),
            secret_key: Secret::new(secret.clone()),
        });
    }
    match get("PT_SECRETS_FILE") {
        Some(path) => load_from_file(path),
        None => Err(ConfigError::MissingCredentials),
    }
}

#[derive(serde::Deserialize)]
struct SecretsFile {
    key_id: String,
    secret_key: String,
}

fn load_from_file(path: &str) -> Result<ApiKeys, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
    // Parse errors may quote file content, so report only the location-free kind.
    let file: SecretsFile = toml::from_str(&text)
        .map_err(|_| ConfigError::Parse("secrets file is not valid".to_string()))?;
    if file.key_id.is_empty() || file.secret_key.is_empty() {
        return Err(ConfigError::MissingCredentials);
    }
    Ok(ApiKeys {
        key_id: Secret::new(file.key_id),
        secret_key: Secret::new(file.secret_key),
    })
}
