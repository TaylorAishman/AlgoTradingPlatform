//! Deflated Sharpe Ratio (Bailey and Lopez de Prado).

use crate::error::StatsError;
use crate::normal::{norm_cdf, norm_inv_cdf};

/// Euler-Mascheroni constant as used in the paper.
const EULER_GAMMA: f64 = std::f64::consts::EULER_GAMMA;

/// Expected maximum per-period Sharpe ratio over `n_trials` independent trials whose
/// Sharpe ratios have sample variance `var_sr`. Returns exactly 0.0 for 0 or 1 trials.
pub fn expected_max_sharpe(var_sr: f64, n_trials: u32) -> f64 {
    if n_trials <= 1 {
        return 0.0;
    }
    let n = f64::from(n_trials);
    let a = norm_inv_cdf(1.0 - 1.0 / n).unwrap_or(0.0);
    let b = norm_inv_cdf(1.0 - 1.0 / (n * std::f64::consts::E)).unwrap_or(0.0);
    var_sr.sqrt() * ((1.0 - EULER_GAMMA) * a + EULER_GAMMA * b)
}

/// Probabilistic Sharpe Ratio: the probability that the true Sharpe ratio exceeds
/// `sr_benchmark`, given the observed per-period `sr` over `t` observations with `skew` and
/// (non-excess) `kurt`. Fails with `TooFewObservations` when `t < 2` and `OutOfDomain` when the
/// variance term under the square root is not positive.
pub fn psr(sr: f64, t: usize, skew: f64, kurt: f64, sr_benchmark: f64) -> Result<f64, StatsError> {
    if t < 2 {
        return Err(StatsError::TooFewObservations);
    }
    let variance_term = 1.0 - skew * sr + (kurt - 1.0) / 4.0 * sr * sr;
    if variance_term.is_nan() || variance_term <= 0.0 {
        return Err(StatsError::OutOfDomain(format!(
            "non-positive variance term {variance_term}"
        )));
    }
    let denom = variance_term.sqrt();
    let z = (sr - sr_benchmark) * ((t as f64) - 1.0).sqrt() / denom;
    Ok(norm_cdf(z))
}

/// Deflated Sharpe Ratio: the PSR against the benchmark `expected_max_sharpe(var_sr, n_trials)`.
pub fn deflated_sharpe(
    sr: f64,
    t: usize,
    skew: f64,
    kurt: f64,
    var_sr: f64,
    n_trials: u32,
) -> Result<f64, StatsError> {
    psr(sr, t, skew, kurt, expected_max_sharpe(var_sr, n_trials))
}
