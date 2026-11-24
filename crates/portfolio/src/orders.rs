//! Whole-share targets and the order plan.

use crate::error::PortfolioError;
use pt_core::money::{Money, Price};
use pt_core::types::{Positions, Symbol, Weights};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Whole-share targets: `weight * equity / price`, truncated toward zero.
///
/// A zero weight gives 0 shares. A non-zero weight without a price is `MissingPrice`.
pub fn target_shares(
    weights: &Weights,
    equity: Money,
    prices: &BTreeMap<Symbol, Price>,
) -> Result<Positions, PortfolioError> {
    let mut out = Positions::new();
    for (symbol, weight) in weights {
        if weight.is_zero() {
            out.insert(symbol.clone(), 0);
            continue;
        }
        let price = prices
            .get(symbol)
            .ok_or_else(|| PortfolioError::MissingPrice(symbol.as_str().to_string()))?;
        let raw: Decimal = (*weight)
            .checked_mul(equity.0)
            .and_then(|notional| notional.checked_div(price.value()))
            .ok_or_else(|| PortfolioError::Invalid(format!("share count overflow for {symbol}")))?;
        let shares = raw
            .trunc()
            .to_i64()
            .ok_or_else(|| PortfolioError::Invalid(format!("share count overflow for {symbol}")))?;
        out.insert(symbol.clone(), shares);
    }
    Ok(out)
}
