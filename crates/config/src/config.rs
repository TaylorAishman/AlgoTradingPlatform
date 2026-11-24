//! The typed configuration. Unknown keys are rejected.

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer};

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

/// Where the run stores its database and reports.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunCfg {
    pub db_path: String,
    pub output_dir: String,
}

impl Default for RunCfg {
    fn default() -> Self {
        Self {
            db_path: "var/paper-trader.sqlite".into(),
            output_dir: "var/reports".into(),
        }
    }
}

/// The market data source.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataCfg {
    pub feed: String,
    #[serde(deserialize_with = "date_from_str")]
    pub start: NaiveDate,
    pub max_requests_per_minute: u32,
}

impl Default for DataCfg {
    fn default() -> Self {
        Self {
            feed: "sip".into(),
            start: NaiveDate::from_ymd_opt(2016, 1, 4).expect("valid date"),
            max_requests_per_minute: 180,
        }
    }
}

fn date_from_str<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDate, D::Error> {
    let text = String::deserialize(d)?;
    text.parse().map_err(serde::de::Error::custom)
}

/// The momentum strategy settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StrategyCfg {
    pub id: String,
    pub lookback_months: Vec<u32>,
    pub risk_free_symbol: String,
    pub vol_com_days: u32,
    pub vol_annualization: u32,
    pub vol_min_obs: u32,
    pub vol_max_obs: u32,
}

impl Default for StrategyCfg {
    fn default() -> Self {
        Self {
            id: "tsmom-v1".into(),
            lookback_months: vec![12],
            risk_free_symbol: "BIL".into(),
            vol_com_days: 60,
            vol_annualization: 261,
            vol_min_obs: 126,
            vol_max_obs: 504,
        }
    }
}

/// Statistics settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatsCfg {
    pub periods_per_year: u32,
    pub pbo_partitions: u32,
}

impl Default for StatsCfg {
    fn default() -> Self {
        Self {
            periods_per_year: 252,
            pbo_partitions: 16,
        }
    }
}

/// The whole configuration (sizing, risk, cost and signoff come later).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub run: RunCfg,
    #[serde(default)]
    pub data: DataCfg,
    #[serde(default)]
    pub strategy: StrategyCfg,
    #[serde(default)]
    pub stats: StatsCfg,
    pub universe: Vec<Instrument>,
}

/// Parse configuration text.
pub fn parse(toml_text: &str) -> Result<Config, ConfigError> {
    toml::from_str(toml_text).map_err(|e| ConfigError::Parse(e.to_string()))
}
