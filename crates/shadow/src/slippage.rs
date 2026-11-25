//! Slippage of a fill against a reference price.

use pt_core::types::Side;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Slippage in basis points, positive when the fill is worse than the reference.
///
/// A buy is worse when filled above the reference, a sell when filled below it.
/// A zero reference price gives zero.
pub fn slippage_bps(side: Side, fill: Decimal, reference: Decimal) -> f64 {
    if reference.is_zero() {
        return 0.0;
    }
    let diff = match side {
        Side::Buy => fill - reference,
        Side::Sell => reference - fill,
    };
    (diff / reference * Decimal::from(10_000))
        .to_f64()
        .unwrap_or(0.0)
}
