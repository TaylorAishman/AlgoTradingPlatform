//! Whole-share targets and the order plan.

use crate::error::PortfolioError;
use pt_core::money::{Money, Price};
use pt_core::types::{OrderIntent, Positions, Side, Symbol, Weights};
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

/// Orders that trade the difference between current and target shares.
///
/// One leg-1 order per target symbol whose share count changes, sorted by symbol.
/// The side follows the sign of `target - current`; the reference price comes from `prices`.
pub fn plan_orders(
    current: &Positions,
    target: &Positions,
    prices: &BTreeMap<Symbol, Price>,
) -> Result<Vec<OrderIntent>, PortfolioError> {
    let mut out = Vec::new();
    for (symbol, want) in target {
        let have = current.get(symbol).copied().unwrap_or(0);
        let diff = want.checked_sub(have).ok_or_else(|| {
            PortfolioError::Invalid(format!("share difference overflow for {symbol}"))
        })?;
        if diff == 0 {
            continue;
        }
        let price = *prices
            .get(symbol)
            .ok_or_else(|| PortfolioError::MissingPrice(symbol.as_str().to_string()))?;
        let side = if diff > 0 { Side::Buy } else { Side::Sell };
        let order = OrderIntent::new(symbol.clone(), side, diff.unsigned_abs(), 1, price)
            .map_err(|e| PortfolioError::Invalid(e.to_string()))?;
        out.push(order);
    }
    Ok(out)
}
