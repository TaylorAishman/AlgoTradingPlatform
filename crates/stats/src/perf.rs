//! Performance metrics over `f64` return series.

use crate::descriptive::{mean, std_sample};

/// Per-period Sharpe ratio: mean over sample std; `0.0` when there is no variation.
pub fn sharpe(returns: &[f64]) -> f64 {
    let sd = std_sample(returns);
    if sd == 0.0 {
        0.0
    } else {
        mean(returns) / sd
    }
}

/// Sharpe ratio scaled by the square root of the periods per year.
pub fn annualized_sharpe(returns: &[f64], periods_per_year: f64) -> f64 {
    sharpe(returns) * periods_per_year.sqrt()
}

/// Sample standard deviation scaled by the square root of the periods per year.
pub fn annualized_vol(returns: &[f64], periods_per_year: f64) -> f64 {
    std_sample(returns) * periods_per_year.sqrt()
}

/// Largest peak-to-trough fall of an equity curve, as a fraction of the peak (`0.0` if none).
pub fn max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut worst = 0.0_f64;
    for &e in equity {
        if e.is_nan() {
            return f64::NAN;
        }
        peak = peak.max(e);
        if peak > 0.0 {
            worst = worst.max((peak - e) / peak);
        }
    }
    worst
}

/// Compound annual growth rate of an equity curve sampled `periods_per_year` times a year.
///
/// Returns NaN when any value is NaN, and `0.0` when there are fewer than two points or the start or end value is not positive.
pub fn cagr(equity: &[f64], periods_per_year: f64) -> f64 {
    let (Some(&first), Some(&last)) = (equity.first(), equity.last()) else {
        return 0.0;
    };
    if equity.iter().any(|e| e.is_nan()) {
        return f64::NAN;
    }
    if equity.len() < 2 || first <= 0.0 || last <= 0.0 {
        return 0.0;
    }
    let years = (equity.len() - 1) as f64 / periods_per_year;
    (last / first).powf(1.0 / years) - 1.0
}
