//! Target weights: inverse-volatility positions scaled to the portfolio volatility target.

use std::collections::{BTreeMap, BTreeSet};

use pt_core::money::round6;
use pt_core::signal::RiskEstimates;
use pt_core::types::{Symbol, Weights};
use rust_decimal::Decimal;

/// Sizing parameters for [`target_weights`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizingParams {
    pub target_vol: f64,
    pub max_weight: f64,
    pub max_gross: f64,
}

/// Covariance of `a` and `b`, accepting either key order.
fn cov_of(risk: &RiskEstimates, a: &Symbol, b: &Symbol) -> Option<f64> {
    risk.cov
        .get(&(a.clone(), b.clone()))
        .or_else(|| risk.cov.get(&(b.clone(), a.clone())))
        .copied()
}

fn to_decimal(x: f64) -> Decimal {
    if !x.is_finite() {
        return Decimal::ZERO;
    }
    Decimal::from_scientific(&format!("{x:e}")).unwrap_or(Decimal::ZERO)
}

/// Inverse-volatility weights scaled so the portfolio volatility equals `target_vol`.
///
/// `u_i = signal_i / vol_i`, `w_i = u_i * target_vol / sqrt(u' C u)`, each clamped to
/// `[-max_weight, max_weight]`, rounded to 6 places.
/// A symbol without a positive volatility gets weight 0. If the portfolio volatility is
/// zero, or a covariance entry between two sized symbols is missing, every weight is 0:
/// no position is taken on risk that cannot be measured.
pub fn target_weights(
    signals: &BTreeMap<Symbol, f64>,
    risk: &RiskEstimates,
    _shortable: &BTreeSet<Symbol>,
    p: &SizingParams,
) -> Weights {
    let zeros: Weights = signals.keys().map(|s| (s.clone(), Decimal::ZERO)).collect();

    let u: Vec<(&Symbol, f64)> = signals
        .iter()
        .filter_map(|(s, &sig)| {
            let vol = *risk.vols.get(s)?;
            (vol.is_finite() && vol > 0.0 && sig.is_finite()).then_some((s, sig / vol))
        })
        .collect();

    let mut variance = 0.0;
    for (a, ua) in &u {
        for (b, ub) in &u {
            match cov_of(risk, a, b) {
                Some(c) => variance += ua * ub * c,
                None => return zeros,
            }
        }
    }
    if !(variance.is_finite() && variance > 0.0) {
        return zeros;
    }
    let scale = p.target_vol / variance.sqrt();

    let cap = to_decimal(p.max_weight.abs());
    let mut weights = zeros;
    for (s, us) in u {
        let w = to_decimal(us * scale).max(-cap).min(cap);
        weights.insert(s.clone(), round6(w));
    }
    weights
}
