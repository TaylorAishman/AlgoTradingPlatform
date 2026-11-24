//! Deflated Sharpe Ratio against the worked example of Bailey and Lopez de Prado.

use pt_stats::dsr::{deflated_sharpe, expected_max_sharpe, psr};

const T: usize = 1250;
const SKEW: f64 = -3.0;
const KURT: f64 = 10.0;
const VAR_SR: f64 = 0.002;

fn sr() -> f64 {
    2.5 / 250f64.sqrt()
}

// R-237
#[test]
fn deflated_sharpe_matches_paper_with_100_trials() {
    let v = deflated_sharpe(sr(), T, SKEW, KURT, VAR_SR, 100).unwrap();
    assert!((v - 0.900397).abs() < 1e-5, "got {v}");
}

// R-237
#[test]
fn deflated_sharpe_matches_paper_with_46_trials() {
    let v = deflated_sharpe(sr(), T, SKEW, KURT, VAR_SR, 46).unwrap();
    assert!((v - 0.950502).abs() < 1e-5, "got {v}");
}

// R-237
#[test]
fn deflated_sharpe_normal_returns_with_88_trials() {
    let v = deflated_sharpe(sr(), T, 0.0, 3.0, VAR_SR, 88).unwrap();
    assert!((v - 0.950491).abs() < 1e-5, "got {v}");
}

// R-238
#[test]
fn psr_equals_deflated_sharpe_at_expected_max_benchmark() {
    for n in [100u32, 46] {
        let sr0 = expected_max_sharpe(VAR_SR, n);
        let p = psr(sr(), T, SKEW, KURT, sr0).unwrap();
        let d = deflated_sharpe(sr(), T, SKEW, KURT, VAR_SR, n).unwrap();
        assert!((p - d).abs() < 1e-12, "n={n}: psr {p} vs dsr {d}");
    }
}

// R-238
#[test]
fn psr_at_its_own_sharpe_is_one_half() {
    let p = psr(0.1, 100, 0.0, 3.0, 0.1).unwrap();
    assert!((p - 0.5).abs() < 1e-12, "got {p}");
}
