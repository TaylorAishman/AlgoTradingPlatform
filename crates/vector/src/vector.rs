//! The vectorised equity calculation.

use crate::error::VectorError;
use chrono::NaiveDate;
use pt_core::cost::CostSpec;
use pt_core::money::round6;
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

const TEN_THOUSAND: Decimal = Decimal::from_parts(10_000, 0, 0, false, 0);
const DAYS_IN_YEAR: Decimal = Decimal::from_parts(360, 0, 0, false, 0);

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

/// Equity at each date's close, with spread, fee and borrow costs. Inputs are validated first.
///
/// On a trade date the target shares are sized from the previous close and
/// the previous close's equity, truncated toward zero, and filled at the
/// open. Positions are marked to each close. The first date has no previous
/// close, so a weights entry there is not traded.
pub fn vector_equity(inp: &VectorInputs) -> Result<Vec<Decimal>, VectorError> {
    validate(inp)?;
    let m = inp.symbols.len();
    let bps = inp.cost.spread_bps + inp.cost.fee_bps;
    let mut cash = inp.initial_capital;
    let mut shares = vec![Decimal::ZERO; m];
    let mut prev_equity = inp.initial_capital;
    let mut out = Vec::with_capacity(inp.dates.len());
    for (i, date) in inp.dates.iter().enumerate() {
        if i > 0 {
            if let Some(w) = inp.weights.get(date) {
                for j in 0..m {
                    let target = (w[j] * prev_equity / inp.close[i - 1][j]).trunc();
                    let open = inp.open[i][j];
                    // Opposite signs trade as two fills: close, then open.
                    let flips = shares[j] * target < Decimal::ZERO;
                    let legs = if flips {
                        vec![-shares[j], target]
                    } else {
                        vec![target - shares[j]]
                    };
                    for delta in legs {
                        cash -= delta * open;
                        cash -= round6(delta.abs() * open * bps / TEN_THOUSAND);
                    }
                    shares[j] = target;
                }
            }
        }
        let short_value: Decimal = shares
            .iter()
            .zip(&inp.close[i])
            .filter(|(s, _)| **s < Decimal::ZERO)
            .map(|(s, p)| -s * p)
            .sum();
        cash -= round6(
            short_value * inp.cost.borrow_bps_annual / TEN_THOUSAND / DAYS_IN_YEAR
                * Decimal::from(inp.days_to_next[i]),
        );
        let held: Decimal = shares.iter().zip(&inp.close[i]).map(|(s, p)| s * p).sum();
        prev_equity = cash + held;
        out.push(prev_equity);
    }
    Ok(out)
}
