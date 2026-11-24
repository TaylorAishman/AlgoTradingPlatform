//! Tests for the Probabilistic Sharpe Ratio (R-238).
//!
//! `deflated_sharpe` is added by a later step, so the equality with it is pinned through the
//! paper's published values (R-237), which `deflated_sharpe` must also reproduce.

use pt_stats::dsr::{expected_max_sharpe, psr};

const TOL: f64 = 1e-5;

fn paper_sr() -> f64 {
    2.5 / 250f64.sqrt()
}

// R-238
#[test]
fn psr_with_equal_sharpe_and_benchmark_is_one_half() {
    let p = psr(0.1, 100, 0.0, 3.0, 0.1).expect("valid inputs");
    assert!((p - 0.5).abs() <= 1e-9, "got {p}");
}

// R-238
#[test]
fn psr_against_expected_max_of_100_trials_matches_paper_dsr() {
    let sr0 = expected_max_sharpe(0.002, 100);
    let p = psr(paper_sr(), 1250, -3.0, 10.0, sr0).expect("valid inputs");
    assert!((p - 0.900397).abs() <= TOL, "got {p}");
}

// R-238
#[test]
fn psr_against_expected_max_of_46_trials_matches_paper_dsr() {
    let sr0 = expected_max_sharpe(0.002, 46);
    let p = psr(paper_sr(), 1250, -3.0, 10.0, sr0).expect("valid inputs");
    assert!((p - 0.950502).abs() <= TOL, "got {p}");
}

// R-238
#[test]
fn psr_for_normal_returns_against_expected_max_of_88_trials_matches_paper_dsr() {
    let sr0 = expected_max_sharpe(0.002, 88);
    let p = psr(paper_sr(), 1250, 0.0, 3.0, sr0).expect("valid inputs");
    assert!((p - 0.950491).abs() <= TOL, "got {p}");
}

// R-238
#[test]
fn psr_rises_when_benchmark_falls() {
    let low = psr(0.1, 100, 0.0, 3.0, 0.0).expect("valid inputs");
    let high = psr(0.1, 100, 0.0, 3.0, 0.2).expect("valid inputs");
    assert!(low > 0.5 && high < 0.5, "low {low}, high {high}");
}
