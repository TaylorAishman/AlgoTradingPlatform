//! The vectorised equity calculation.

use crate::error::VectorError;
use chrono::NaiveDate;
use pt_core::cost::CostSpec;
use pt_core::types::Symbol;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Everything the calculator needs, laid out `[date][symbol]`.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorInputs {
    pub dates: Vec<NaiveDate>,
    pub symbols: Vec<Symbol>,
    pub open: Vec<Vec<Decimal>>,
    pub close: Vec<Vec<Decimal>>,
    /// Target weights by trade date.
    pub weights: BTreeMap<NaiveDate, Vec<Decimal>>,
    pub days_to_next: Vec<i64>,
    pub initial_capital: Decimal,
    pub cost: CostSpec,
}

fn validate(inp: &VectorInputs) -> Result<(), VectorError> {
    let n = inp.dates.len();
    let m = inp.symbols.len();
    if inp.open.len() != n || inp.close.len() != n {
        return Err(VectorError::Shape(
            "price rows differ from the date count".into(),
        ));
    }
    for (name, grid) in [("open", &inp.open), ("close", &inp.close)] {
        if grid.iter().any(|r| r.len() != m) {
            return Err(VectorError::Shape(format!(
                "a {name} row differs from the symbol count"
            )));
        }
    }
    for (date, w) in &inp.weights {
        if !inp.dates.contains(date) {
            return Err(VectorError::UnknownTradeDate(*date));
        }
        if w.len() != m {
            return Err(VectorError::Shape(format!(
                "weights row for {date} differs from the symbol count"
            )));
        }
    }
    if inp
        .open
        .iter()
        .chain(inp.close.iter())
        .flatten()
        .any(|p| *p <= Decimal::ZERO)
    {
        return Err(VectorError::NonPositivePrice);
    }
    if !inp.cost.impact_coeff.is_zero() {
        return Err(VectorError::ImpactNotSupported);
    }
    if inp.days_to_next.len() != n {
        return Err(VectorError::LengthMismatch);
    }
    Ok(())
}

/// Equity at each date's close. Inputs are validated first.
///
/// The accounting is added in a later step; valid inputs currently
/// return the initial capital for every date.
pub fn vector_equity(inp: &VectorInputs) -> Result<Vec<Decimal>, VectorError> {
    validate(inp)?;
    Ok(vec![inp.initial_capital; inp.dates.len()])
}
