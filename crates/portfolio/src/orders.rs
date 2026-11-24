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
/// One leg-1 order per symbol whose share count changes. A sign flip is
/// two orders: leg 1 closes to zero, leg 2 opens the new side. A held symbol
/// absent from `target` is closed to zero.
/// Output order: leg 1 sells, leg 1 buys, leg 2 sells, leg 2 buys, each group sorted by symbol.
/// The side follows the sign of `target - current`; the reference price comes from `prices`.
pub fn plan_orders(
    current: &Positions,
    target: &Positions,
    prices: &BTreeMap<Symbol, Price>,
) -> Result<Vec<OrderIntent>, PortfolioError> {
    let mut symbols: std::collections::BTreeSet<&Symbol> = target.keys().collect();
    symbols.extend(current.keys());
    let mut out = Vec::new();
    for symbol in symbols {
        let want = target.get(symbol).copied().unwrap_or(0);
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
        let flips = (have > 0 && want < 0) || (have < 0 && want > 0);
        let legs: [(u64, u8); 2] = if flips {
            [(have.unsigned_abs(), 1), (want.unsigned_abs(), 2)]
        } else {
            [(diff.unsigned_abs(), 1), (0, 0)]
        };
        for (qty, leg) in legs {
            if qty == 0 {
                continue;
            }
            let order = OrderIntent::new(symbol.clone(), side, qty, leg, price)
                .map_err(|e| PortfolioError::Invalid(e.to_string()))?;
            out.push(order);
        }
    }
    // Stable sort keeps symbols sorted within each (leg, side) group.
    out.sort_by_key(|o| (o.leg, o.side != Side::Sell));
    Ok(out)
}
