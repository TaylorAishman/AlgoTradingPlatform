//! Cost functions.

use crate::error::EngineError;
use pt_core::cost::CostSpec;
use pt_core::money::{round6, Money, Price};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

const BPS: i64 = 10_000;

/// Inputs for the square-root market impact term: daily volatility as a
/// fraction and average daily volume in shares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactInputs {
    pub daily_vol: f64,
    pub adv_shares: f64,
}

/// Cost of one fill: `round6(notional * (spread + fee) / 10000)` plus
/// `round6(notional * impact_coeff * daily_vol * sqrt(qty / adv_shares))`.
///
/// The cost is always non-negative. `impact` is only needed when
/// `impact_coeff` is non-zero; then inputs that are `None`, non-finite,
/// negative, or have zero `adv_shares` are `MissingImpactInputs`.
pub fn fill_cost(
    spec: &CostSpec,
    qty: u64,
    price: Price,
    impact: Option<ImpactInputs>,
) -> Result<Money, EngineError> {
    spec.validate()?;
    let shares = i64::try_from(qty)
        .map_err(|_| EngineError::Invalid(format!("quantity {qty} is too large")))?;
    let notional = price.notional(shares).0.abs();
    let spread_fee = round6(notional * (spec.spread_bps + spec.fee_bps) / Decimal::from(BPS));
    let mut impact_cost = Decimal::ZERO;
    if !spec.impact_coeff.is_zero() {
        let (vol, adv) = impact
            .and_then(|i| {
                let ok = i.daily_vol.is_finite()
                    && i.adv_shares.is_finite()
                    && i.daily_vol >= 0.0
                    && i.adv_shares > 0.0;
                if !ok {
                    return None;
                }
                Some((
                    Decimal::from_f64(i.daily_vol)?,
                    Decimal::from_f64(i.adv_shares)?,
                ))
            })
            .filter(|(_, adv)| !adv.is_zero())
            .ok_or_else(|| EngineError::MissingImpactInputs("daily_vol, adv_shares".to_string()))?;
        let participation = Decimal::from(qty) / adv;
        impact_cost = round6(spec.impact_coeff * vol * decimal_sqrt(participation) * notional);
    }
    Ok(Money(spread_fee + impact_cost))
}

/// Borrow cost on a short market value over `days` calendar days:
/// `short_value * borrow_bps_annual / 10000 * days / 360`, rounded to six places.
pub fn borrow_cost(spec: &CostSpec, short_value: Money, days: i64) -> Money {
    let annual = short_value.0 * spec.borrow_bps_annual / Decimal::from(BPS);
    Money(round6(annual * Decimal::from(days) / Decimal::from(360)))
}

/// Square root of a non-negative decimal by Newton iteration. Returns an
/// exact root when one exists at fewer digits than the converged value.
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let two = Decimal::from(2);
    let mut g = if x > Decimal::ONE {
        x / two
    } else {
        Decimal::ONE
    };
    for _ in 0..100 {
        let next = (g + x / g) / two;
        if next == g {
            break;
        }
        g = next;
    }
    for dp in 0..=g.scale() {
        let r = g.round_dp(dp);
        if r.checked_mul(r) == Some(x) {
            return r;
        }
    }
    g
}
