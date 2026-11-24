//! Tests for the inverse normal distribution function (R-235).

use pt_stats::error::StatsError;
use pt_stats::normal::{norm_cdf, norm_inv_cdf};

const TOL: f64 = 1e-8;

fn close(p: f64, expected: f64) {
    let actual = norm_inv_cdf(p).expect("p is inside (0, 1)");
    assert!(
        (actual - expected).abs() <= TOL,
        "norm_inv_cdf({p}) = {actual}, expected {expected}"
    );
}

// R-235
#[test]
fn norm_inv_cdf_of_0_975_matches_reference() {
    close(0.975, 1.9599639845);
}

// R-235
#[test]
fn norm_inv_cdf_of_one_half_is_zero() {
    close(0.5, 0.0);
}

// R-235
#[test]
fn norm_inv_cdf_of_0_99_matches_reference() {
    close(0.99, 2.3263478740);
}

// R-235
#[test]
fn norm_inv_cdf_of_tiny_probability_matches_reference() {
    close(1e-10, -6.3613409024);
}

// R-235
#[test]
fn norm_inv_cdf_is_antisymmetric_about_one_half() {
    for p in [0.01, 0.1, 0.3, 0.45] {
        let lo = norm_inv_cdf(p).unwrap();
        let hi = norm_inv_cdf(1.0 - p).unwrap();
        assert!((lo + hi).abs() <= TOL, "p = {p}: {lo} vs {hi}");
    }
}

// R-235
#[test]
fn norm_inv_cdf_inverts_norm_cdf() {
    for p in [1e-6, 0.02, 0.2, 0.5, 0.8, 0.98] {
        let x = norm_inv_cdf(p).unwrap();
        assert!((norm_cdf(x) - p).abs() <= 1e-8, "p = {p}");
    }
}

// R-235
#[test]
fn norm_inv_cdf_rejects_values_outside_open_unit_interval() {
    for p in [0.0, 1.0, 1.5, -0.1, f64::NAN] {
        assert!(
            matches!(norm_inv_cdf(p), Err(StatsError::OutOfDomain(_))),
            "p = {p}"
        );
    }
}
