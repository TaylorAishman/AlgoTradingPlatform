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
