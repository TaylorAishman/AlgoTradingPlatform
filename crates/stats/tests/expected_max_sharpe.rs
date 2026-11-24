//! Tests for the expected maximum Sharpe ratio (R-236).

use pt_stats::dsr::expected_max_sharpe;

const TOL: f64 = 1e-6;

fn close(var_sr: f64, n_trials: u32, expected: f64) {
    let actual = expected_max_sharpe(var_sr, n_trials);
    assert!(
        (actual - expected).abs() <= TOL,
        "expected_max_sharpe({var_sr}, {n_trials}) = {actual}, expected {expected}"
    );
}

// R-236
#[test]
fn expected_max_sharpe_for_100_trials_matches_paper() {
    close(0.002, 100, 0.113172);
}

// R-236
#[test]
fn expected_max_sharpe_for_46_trials_matches_paper() {
    close(0.002, 46, 0.100362);
}

// R-236
#[test]
fn expected_max_sharpe_with_one_trial_is_zero() {
    assert_eq!(expected_max_sharpe(0.002, 1), 0.0);
}

// R-236
#[test]
fn expected_max_sharpe_with_no_trials_is_zero() {
    assert_eq!(expected_max_sharpe(0.002, 0), 0.0);
}
