//! Cost functions.

use crate::error::EngineError;
use pt_core::cost::CostSpec;
use pt_core::money::{round6, Money, Price};
use rust_decimal::Decimal;

const BPS: i64 = 10_000;

/// Inputs for the market impact term. Not used until the impact model exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactInputs {
    pub daily_vol: f64,
    pub adv_shares: f64,
}

/// Cost of one fill: (spread bps + fee bps) of the traded notional.
///
/// The cost is always non-negative. `impact` is ignored when `None`.
pub fn fill_cost(
    spec: &CostSpec,
    qty: u64,
    price: Price,
    _impact: Option<ImpactInputs>,
) -> Result<Money, EngineError> {
    spec.validate()?;
    let shares = i64::try_from(qty)
        .map_err(|_| EngineError::Invalid(format!("quantity {qty} is too large")))?;
    let notional = price.notional(shares).0.abs();
    let spread_fee = round6(notional * (spec.spread_bps + spec.fee_bps) / Decimal::from(BPS));
    Ok(Money(spread_fee))
}
