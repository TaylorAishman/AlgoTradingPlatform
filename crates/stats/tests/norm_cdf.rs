//! Tests for the normal distribution function (R-234).

use pt_stats::normal::norm_cdf;

const TOL: f64 = 1e-9;

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL,
        "got {actual}, expected {expected}"
    );
}

// R-234
#[test]
fn norm_cdf_of_zero_is_one_half() {
    close(norm_cdf(0.0), 0.5);
}

// R-234
#[test]
fn norm_cdf_of_one_matches_reference() {
    close(norm_cdf(1.0), 0.8413447461);
}

// R-234
#[test]
fn norm_cdf_of_minus_1_96_matches_reference() {
    close(norm_cdf(-1.96), 0.0249978952);
}

// R-234
#[test]
fn norm_cdf_is_symmetric_about_zero() {
    for x in [0.1, 0.5, 1.0, 2.0, 3.5] {
        close(norm_cdf(x) + norm_cdf(-x), 1.0);
    }
}

// R-234
#[test]
fn norm_cdf_tails_approach_zero_and_one() {
    close(norm_cdf(10.0), 1.0);
    close(norm_cdf(-10.0), 0.0);
}
