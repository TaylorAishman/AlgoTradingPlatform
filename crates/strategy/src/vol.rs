//! Exponentially weighted volatility (Arch 5.10.1, Moskowitz et al., equation 1).

/// Annualised exponentially weighted volatility of `returns`, ordered oldest first.
///
/// With `delta = com / (1 + com)` the newest return has weight `(1 - delta)` and
/// each older one is multiplied by a further `delta`. Weights are normalised by
/// their sum, the finite-sample form of the paper's estimator. Returns `None`
/// when fewer than two returns are given.
pub fn ewma_vol(returns: &[f64], com: f64, annualization: f64) -> Option<f64> {
    if returns.len() < 2 {
        return None;
    }
    let delta = com / (1.0 + com);
    let weights: Vec<f64> = (0..returns.len())
        .map(|i| (1.0 - delta) * delta.powi(i as i32))
        .collect();
    let total: f64 = weights.iter().sum();
    // Index 0 is the newest return, which is the last element of the slice.
    let newest_first = returns.iter().rev();
    let mean: f64 = weights
        .iter()
        .zip(newest_first.clone())
        .map(|(w, r)| w * r)
        .sum::<f64>()
        / total;
    let var: f64 = weights
        .iter()
        .zip(newest_first)
        .map(|(w, r)| w * (r - mean).powi(2))
        .sum::<f64>()
        / total;
    Some((annualization * var).sqrt())
}

/// Annualised exponentially weighted covariance of two return series, ordered oldest first.
///
/// Uses the same weights, normalisation and weighted-mean centring as
/// [`ewma_vol`], so the covariance of a series with itself is its volatility
/// squared. Returns `None` when the lengths differ or fewer than two returns
/// are given.
pub fn ewma_cov(a: &[f64], b: &[f64], com: f64, annualization: f64) -> Option<f64> {
    if a.len() != b.len() || a.len() < 2 {
        return None;
    }
    let delta = com / (1.0 + com);
    let weights: Vec<f64> = (0..a.len())
        .map(|i| (1.0 - delta) * delta.powi(i as i32))
        .collect();
    let total: f64 = weights.iter().sum();
    // Index 0 is the newest return, which is the last element of the slice.
    let weighted_mean = |x: &[f64]| -> f64 {
        weights
            .iter()
            .zip(x.iter().rev())
            .map(|(w, r)| w * r)
            .sum::<f64>()
            / total
    };
    let (mean_a, mean_b) = (weighted_mean(a), weighted_mean(b));
    let cov: f64 = weights
        .iter()
        .zip(a.iter().rev().zip(b.iter().rev()))
        .map(|(w, (x, y))| w * (x - mean_a) * (y - mean_b))
        .sum::<f64>()
        / total;
    Some(annualization * cov)
}
