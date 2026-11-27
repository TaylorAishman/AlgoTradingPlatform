//! R-465: a statistic that returns a plain number returns NaN when an input is NaN.

use pt_stats::descriptive::{kurtosis, mean, skewness, std_sample};
use pt_stats::perf::{annualized_sharpe, annualized_vol, cagr, max_drawdown, sharpe};

const RETURNS: [f64; 6] = [0.01, -0.02, 0.015, f64::NAN, 0.005, -0.01];

#[test]
fn mean_is_nan_with_a_nan_input() {
    // R-465
    assert!(mean(&RETURNS).is_nan());
}

#[test]
fn std_sample_is_nan_with_a_nan_input() {
    // R-465
    assert!(std_sample(&RETURNS).is_nan());
}

#[test]
fn skewness_is_nan_with_a_nan_input() {
    // R-465
    assert!(skewness(&RETURNS).is_nan());
}

#[test]
fn kurtosis_is_nan_with_a_nan_input() {
    // R-465
    assert!(kurtosis(&RETURNS).is_nan());
}

#[test]
fn sharpe_is_nan_with_a_nan_input() {
    // R-465
    assert!(sharpe(&RETURNS).is_nan());
}

#[test]
fn annualized_sharpe_is_nan_with_a_nan_input() {
    // R-465
    assert!(annualized_sharpe(&RETURNS, 252.0).is_nan());
}

#[test]
fn annualized_vol_is_nan_with_a_nan_input() {
    // R-465
    assert!(annualized_vol(&RETURNS, 252.0).is_nan());
}

#[test]
fn max_drawdown_is_nan_with_a_nan_in_the_middle() {
    // R-465: not 0.1
    assert!(max_drawdown(&[100.0, f64::NAN, 90.0]).is_nan());
}

#[test]
fn max_drawdown_is_nan_with_a_nan_first_or_last() {
    // R-465
    assert!(max_drawdown(&[f64::NAN, 100.0, 90.0]).is_nan());
    assert!(max_drawdown(&[100.0, 90.0, f64::NAN]).is_nan());
}

#[test]
fn cagr_is_nan_with_a_nan_input() {
    // R-465
    assert!(cagr(&[100.0, f64::NAN, 120.0, 130.0], 252.0).is_nan());
}
