//! Simple least-squares regression of `y` on `x` with an intercept.

use crate::error::StatsError;

/// Result of a simple regression `y = alpha + beta * x + e`.
#[derive(Debug, Clone, PartialEq)]
pub struct Ols {
    pub alpha: f64,
    pub beta: f64,
    /// t-statistic of the intercept.
    pub alpha_t: f64,
    pub r2: f64,
    pub n: usize,
}

/// Regress `y` on `x`. Needs at least three points and a non-constant `x`.
pub fn ols(y: &[f64], x: &[f64]) -> Result<Ols, StatsError> {
    let n = y.len();
    if n < 3 || x.len() != n {
        return Err(StatsError::TooFewObservations);
    }
    if x.iter().chain(y).any(|v| !v.is_finite()) {
        return Err(StatsError::OutOfDomain(
            "inputs must be finite numbers".to_string(),
        ));
    }
    let nf = n as f64;
    let mx = x.iter().sum::<f64>() / nf;
    let my = y.iter().sum::<f64>() / nf;
    let sxx: f64 = x.iter().map(|v| (v - mx).powi(2)).sum();
    if sxx <= 0.0 || !sxx.is_finite() {
        return Err(StatsError::TooFewObservations);
    }
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let syy: f64 = y.iter().map(|v| (v - my).powi(2)).sum();
    let beta = sxy / sxx;
    let alpha = my - beta * mx;
    let sse: f64 = x
        .iter()
        .zip(y)
        .map(|(a, b)| (b - alpha - beta * a).powi(2))
        .sum();
    let s2 = sse / (nf - 2.0);
    let se_alpha = (s2 * (1.0 / nf + mx * mx / sxx)).sqrt();
    let alpha_t = alpha / se_alpha;
    let r2 = if syy > 0.0 { 1.0 - sse / syy } else { 0.0 };
    Ok(Ols {
        alpha,
        beta,
        alpha_t,
        r2,
        n,
    })
}
