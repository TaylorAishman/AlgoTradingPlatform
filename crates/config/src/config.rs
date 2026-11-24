//! The typed configuration. Unknown keys are rejected.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

use pt_core::cost::CostSpec;
use pt_core::types::Symbol;

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

/// Thresholds for the paper-trading signoff.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SignoffCfg {
    pub min_sessions: u32,
    pub min_rebalances: u32,
    pub min_fills: u32,
    pub max_replay_mismatches: u32,
    pub max_reconciliation_breaks: u32,
    pub max_missed_sessions: u32,
    pub max_late_rebalances: u32,
    pub slippage_tolerance_bps: Decimal,
    pub max_mean_diff_bps: Decimal,
    pub max_abs_diff_bps: Decimal,
}

impl Default for SignoffCfg {
    fn default() -> Self {
        Self {
            min_sessions: 63,
            min_rebalances: 3,
            min_fills: 30,
            max_replay_mismatches: 0,
            max_reconciliation_breaks: 0,
            max_missed_sessions: 2,
            max_late_rebalances: 0,
            slippage_tolerance_bps: Decimal::new(5, 0),
            max_mean_diff_bps: Decimal::ONE,
            max_abs_diff_bps: Decimal::new(25, 0),
        }
    }
}

/// The whole configuration.
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
    #[serde(default)]
    pub signoff: SignoffCfg,
    #[serde(default)]
    pub universe: Vec<Instrument>,
}

/// Parse configuration text.
pub fn parse(toml_text: &str) -> Result<Config, ConfigError> {
    let cfg: Config = toml::from_str(toml_text).map_err(|e| ConfigError::Parse(e.to_string()))?;
    validate(&cfg)?;
    Ok(cfg)
}

/// Read and parse the configuration file at `path`.
pub fn load(path: &std::path::Path) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::Io(format!("{}: {e}", path.display())))?;
    parse(&text)
}

/// Check cross-field and range rules that the types cannot express.
fn validate(cfg: &Config) -> Result<(), ConfigError> {
    let max_gross = cfg.sizing.max_gross;
    if max_gross <= Decimal::ZERO || max_gross > Decimal::ONE {
        return Err(ConfigError::Invalid(format!(
            "sizing.max_gross must be in (0, 1], got {max_gross}"
        )));
    }
    let lookbacks = &cfg.strategy.lookback_months;
    if lookbacks.is_empty() || lookbacks.contains(&0) {
        return Err(ConfigError::Invalid(
            "strategy.lookback_months must be non-empty with every entry at least 1".into(),
        ));
    }
    let mut distinct = std::collections::HashSet::new();
    if !lookbacks.iter().all(|m| distinct.insert(*m)) {
        return Err(ConfigError::Invalid(
            "strategy.lookback_months must not contain duplicates".into(),
        ));
    }
    if cfg.strategy.vol_min_obs > cfg.strategy.vol_max_obs {
        return Err(ConfigError::Invalid(format!(
            "strategy.vol_min_obs ({}) must not exceed strategy.vol_max_obs ({})",
            cfg.strategy.vol_min_obs, cfg.strategy.vol_max_obs
        )));
    }
    if cfg.stats.pbo_partitions < 2 || !cfg.stats.pbo_partitions.is_multiple_of(2) {
        return Err(ConfigError::Invalid(format!(
            "stats.pbo_partitions must be an even number of at least 2, got {}",
            cfg.stats.pbo_partitions
        )));
    }
    if cfg.sizing.target_vol <= Decimal::ZERO {
        return Err(ConfigError::Invalid(format!(
            "sizing.target_vol must be positive, got {}",
            cfg.sizing.target_vol
        )));
    }
    if cfg.risk.max_order_notional <= Decimal::ZERO {
        return Err(ConfigError::Invalid(format!(
            "risk.max_order_notional must be positive, got {}",
            cfg.risk.max_order_notional
        )));
    }
    let fractions = [
        ("sizing.max_weight", cfg.sizing.max_weight),
        ("risk.daily_loss_limit", cfg.risk.daily_loss_limit),
        ("risk.max_drawdown", cfg.risk.max_drawdown),
    ];
    for (key, v) in fractions {
        if v <= Decimal::ZERO || v > Decimal::ONE {
            return Err(ConfigError::Invalid(format!(
                "{key} must be in (0, 1], got {v}"
            )));
        }
    }
    if cfg.risk.cash_tolerance < Decimal::ZERO {
        return Err(ConfigError::Invalid(format!(
            "risk.cash_tolerance must be at least 0, got {}",
            cfg.risk.cash_tolerance
        )));
    }
    let counts = [
        ("stats.periods_per_year", cfg.stats.periods_per_year),
        ("strategy.vol_com_days", cfg.strategy.vol_com_days),
        ("strategy.vol_annualization", cfg.strategy.vol_annualization),
    ];
    for (key, v) in counts {
        if v == 0 {
            return Err(ConfigError::Invalid(format!("{key} must be positive")));
        }
    }
    for (key, spec) in [
        ("cost.research", &cfg.cost.research),
        ("cost.shadow", &cfg.cost.shadow),
    ] {
        spec.validate()
            .map_err(|e| ConfigError::Invalid(format!("{key}: {e}")))?;
    }
    let risk_free = &cfg.strategy.risk_free_symbol;
    Symbol::new(risk_free).map_err(|e| {
        ConfigError::Invalid(format!("strategy.risk_free_symbol \"{risk_free}\": {e}"))
    })?;
    let feed = cfg.data.feed.as_str();
    let imported = feed.strip_prefix("csv:").is_some_and(|n| !n.is_empty());
    if feed != "sip" && feed != "iex" && !imported {
        return Err(ConfigError::Invalid(format!(
            "data.feed must be \"sip\", \"iex\" or \"csv:<name>\", got \"{feed}\""
        )));
    }
    if feed == "iex" {
        if !cfg.cost.research.impact_coeff.is_zero() {
            return Err(ConfigError::Invalid(
                "cost.research.impact_coeff must be 0 when data.feed is \"iex\"".into(),
            ));
        }
        if !cfg.cost.shadow.impact_coeff.is_zero() {
            return Err(ConfigError::Invalid(
                "cost.shadow.impact_coeff must be 0 when data.feed is \"iex\"".into(),
            ));
        }
    }
    if cfg.universe.is_empty() {
        return Err(ConfigError::Invalid("universe must not be empty".into()));
    }
    let mut seen = std::collections::HashSet::new();
    for inst in &cfg.universe {
        if !seen.insert(inst.symbol.as_str()) {
            return Err(ConfigError::Invalid(format!(
                "universe contains duplicate symbol \"{}\"",
                inst.symbol
            )));
        }
        Symbol::new(&inst.symbol).map_err(|e| {
            ConfigError::Invalid(format!("universe symbol \"{}\": {e}", inst.symbol))
        })?;
        if inst.symbol == cfg.strategy.risk_free_symbol {
            return Err(ConfigError::Invalid(format!(
                "universe must not contain the risk-free symbol \"{}\"",
                inst.symbol
            )));
        }
    }
    Ok(())
}
