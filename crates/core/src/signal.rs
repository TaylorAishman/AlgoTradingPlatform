//! Strategy output and risk estimates.

use crate::types::Symbol;
use chrono::NaiveDate;
use std::collections::BTreeMap;

/// Signals produced by a strategy for one decision date.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalSet {
    pub asof: NaiveDate,
    pub signals: BTreeMap<Symbol, f64>,
    pub ineligible: BTreeMap<Symbol, String>,
}

/// Volatilities and pairwise covariances for one decision date.
#[derive(Debug, Clone, PartialEq)]
pub struct RiskEstimates {
    pub asof: NaiveDate,
    pub vols: BTreeMap<Symbol, f64>,
    pub cov: BTreeMap<(Symbol, Symbol), f64>,
    pub ineligible: BTreeMap<Symbol, String>,
}
