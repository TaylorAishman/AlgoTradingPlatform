use pt_stats::descriptive::{kurtosis, skewness};

const EPS: f64 = 1e-9;

fn close(a: f64, b: f64, eps: f64) {
    assert!((a - b).abs() < eps, "{a} != {b}");
}

#[test]
fn skewness_uses_population_moments() {
    // R-231
    close(skewness(&[1.0, 2.0, 3.0, 4.0, 10.0]), 1.1384199577, EPS);
}

#[test]
fn kurtosis_uses_population_moments_with_three_for_normal() {
    // R-231
    close(kurtosis(&[1.0, 2.0, 3.0, 4.0, 10.0]), 2.788, EPS);
}

#[test]
fn constant_series_has_zero_skewness_and_kurtosis() {
    // R-231
    assert_eq!(skewness(&[2.0, 2.0, 2.0, 2.0]), 0.0);
    assert_eq!(kurtosis(&[2.0, 2.0, 2.0, 2.0]), 0.0);
}

#[test]
fn symmetric_series_has_zero_skewness() {
    // R-231
    close(skewness(&[-2.0, -1.0, 0.0, 1.0, 2.0]), 0.0, EPS);
}
