//! R-142: covariance uses the same weights and centring as volatility.

use pt_strategy::vol::{ewma_cov, ewma_vol};

const FIVE: [f64; 5] = [0.01, -0.02, 0.015, 0.005, -0.01];

#[test]
fn two_returns_with_unit_com_match_hand_computed_value() {
    // R-142
    let c = ewma_cov(&[0.02, -0.01], &[0.01, 0.03], 1.0, 261.0).expect("equal lengths");
    assert!((c - -0.0348).abs() < 1e-9, "got {c}");
}

#[test]
fn covariance_of_a_series_with_itself_is_its_volatility_squared() {
    // R-142
    let c = ewma_cov(&FIVE, &FIVE, 60.0, 261.0).expect("equal lengths");
    let v = ewma_vol(&FIVE, 60.0, 261.0).expect("five returns");
    assert!((c - v * v).abs() < 1e-12, "cov {c} vs vol^2 {}", v * v);
}

#[test]
fn series_of_different_lengths_return_none() {
    // R-142
    assert_eq!(
        ewma_cov(&[0.01, 0.02, 0.03], &[0.01, 0.02], 60.0, 261.0),
        None
    );
    assert_eq!(
        ewma_cov(&[0.01, 0.02], &[0.01, 0.02, 0.03], 60.0, 261.0),
        None
    );
}
