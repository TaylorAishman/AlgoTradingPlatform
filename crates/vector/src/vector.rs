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

/// Equity at each date's close at zero cost. Inputs are validated first.
///
/// On a trade date the target shares are sized from the previous close and
/// the previous close's equity, truncated toward zero, and filled at the
/// open. Positions are marked to each close. The first date has no previous
/// close, so a weights entry there is not traded.
pub fn vector_equity(inp: &VectorInputs) -> Result<Vec<Decimal>, VectorError> {
    validate(inp)?;
    let m = inp.symbols.len();
    let mut cash = inp.initial_capital;
    let mut shares = vec![Decimal::ZERO; m];
    let mut prev_equity = inp.initial_capital;
    let mut out = Vec::with_capacity(inp.dates.len());
    for (i, date) in inp.dates.iter().enumerate() {
        if i > 0 {
            if let Some(w) = inp.weights.get(date) {
                for j in 0..m {
                    let target = (w[j] * prev_equity / inp.close[i - 1][j]).trunc();
                    let delta = target - shares[j];
                    cash -= delta * inp.open[i][j];
                    shares[j] = target;
                }
            }
        }
        let held: Decimal = shares.iter().zip(&inp.close[i]).map(|(s, p)| s * p).sum();
        prev_equity = cash + held;
        out.push(prev_equity);
    }
    Ok(out)
}
