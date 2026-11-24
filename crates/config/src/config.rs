//! The typed configuration. Unknown keys are rejected.

use serde::Deserialize;

use crate::error::ConfigError;

/// The asset class of a universe instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    Equity,
    Treasury,
    Credit,
    Commodity,
    Currency,
    RealEstate,
}

/// One tradable instrument in the universe.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub symbol: String,
    pub asset_class: AssetClass,
    pub shortable: bool,
}

/// The whole configuration (only the universe so far).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub universe: Vec<Instrument>,
}

/// Parse configuration text.
pub fn parse(toml_text: &str) -> Result<Config, ConfigError> {
    toml::from_str(toml_text).map_err(|e| ConfigError::Parse(e.to_string()))
}
