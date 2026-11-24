//! The typed configuration. Unknown keys are rejected.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

use pt_core::cost::CostSpec;

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

/// Position sizing limits.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SizingCfg {
    pub target_vol: Decimal,
    pub max_weight: Decimal,
    pub max_gross: Decimal,
}

impl Default for SizingCfg {
    fn default() -> Self {
        Self {
            target_vol: Decimal::new(10, 2),
            max_weight: Decimal::new(10, 2),
            max_gross: Decimal::ONE,
        }
    }
}

/// Pre-trade and portfolio risk limits.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RiskCfg {
    pub max_order_notional: Decimal,
    pub daily_loss_limit: Decimal,
    pub max_drawdown: Decimal,
    pub max_rebalance_delay_sessions: u32,
    pub min_minutes_before_close: u32,
    pub cash_tolerance: Decimal,
}

impl Default for RiskCfg {
    fn default() -> Self {
        Self {
            max_order_notional: Decimal::new(25000, 0),
            daily_loss_limit: Decimal::new(3, 2),
            max_drawdown: Decimal::new(15, 2),
            max_rebalance_delay_sessions: 5,
            min_minutes_before_close: 30,
            cash_tolerance: Decimal::new(1, 2),
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

/// Serde mirror of `CostSpec`; a missing key falls back to the spec's default.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CostSpecRaw {
    spread_bps: Option<Decimal>,
    fee_bps: Option<Decimal>,
    impact_coeff: Option<Decimal>,
    borrow_bps_annual: Option<Decimal>,
}

impl CostSpecRaw {
    fn over(self, base: CostSpec) -> CostSpec {
        CostSpec {
            spread_bps: self.spread_bps.unwrap_or(base.spread_bps),
            fee_bps: self.fee_bps.unwrap_or(base.fee_bps),
            impact_coeff: self.impact_coeff.unwrap_or(base.impact_coeff),
            borrow_bps_annual: self.borrow_bps_annual.unwrap_or(base.borrow_bps_annual),
        }
    }
}

fn research_default() -> CostSpec {
    CostSpec {
        spread_bps: Decimal::new(5, 0),
        fee_bps: Decimal::ZERO,
        impact_coeff: Decimal::ONE,
        borrow_bps_annual: Decimal::new(50, 0),
    }
}

fn shadow_default() -> CostSpec {
    CostSpec {
        spread_bps: Decimal::new(5, 0),
        fee_bps: Decimal::ZERO,
        impact_coeff: Decimal::ZERO,
        borrow_bps_annual: Decimal::ZERO,
    }
}

fn research_spec<'de, D: Deserializer<'de>>(d: D) -> Result<CostSpec, D::Error> {
    Ok(CostSpecRaw::deserialize(d)?.over(research_default()))
}

fn shadow_spec<'de, D: Deserializer<'de>>(d: D) -> Result<CostSpec, D::Error> {
    Ok(CostSpecRaw::deserialize(d)?.over(shadow_default()))
}

/// The two cost models: research (historical backtests) and shadow.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostCfg {
    #[serde(default = "research_default", deserialize_with = "research_spec")]
    pub research: CostSpec,
    #[serde(default = "shadow_default", deserialize_with = "shadow_spec")]
    pub shadow: CostSpec,
}

impl Default for CostCfg {
    fn default() -> Self {
        Self {
            research: research_default(),
            shadow: shadow_default(),
        }
    }
}

/// The whole configuration (signoff comes later).
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
    pub sizing: SizingCfg,
    #[serde(default)]
    pub risk: RiskCfg,
    #[serde(default)]
    pub stats: StatsCfg,
    #[serde(default)]
    pub cost: CostCfg,
    pub universe: Vec<Instrument>,
}

/// Parse configuration text.
pub fn parse(toml_text: &str) -> Result<Config, ConfigError> {
    toml::from_str(toml_text).map_err(|e| ConfigError::Parse(e.to_string()))
}
