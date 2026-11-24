//! Descriptive statistics over `f64` return series.

/// Arithmetic mean; `0.0` for an empty slice.
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Sample standard deviation (n - 1 denominator); `0.0` for fewer than two values.
pub fn std_sample(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let m = mean(xs);
    let ss: f64 = xs.iter().map(|x| (x - m).powi(2)).sum();
    (ss / (xs.len() - 1) as f64).sqrt()
}

/// Simple period-over-period returns from an equity curve.
pub fn returns_from_equity(equity: &[f64]) -> Vec<f64> {
    equity.windows(2).map(|w| w[1] / w[0] - 1.0).collect()
}
