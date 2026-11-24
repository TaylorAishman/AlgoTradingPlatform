//! R-140: volatility is the exponentially weighted estimate of Arch 5.10.1.

use pt_strategy::vol::ewma_vol;

const FIVE: [f64; 5] = [0.01, -0.02, 0.015, 0.005, -0.01];

#[test]
fn two_returns_with_unit_com_match_hand_computed_value() {
    // R-140
    let v = ewma_vol(&[0.02, -0.01], 1.0, 261.0).expect("two returns give an estimate");
    assert!((v - 0.2284731932).abs() < 1e-9, "got {v}");
}

#[test]
fn five_returns_with_sixty_day_com_match_reference_value() {
    // R-140
    let v = ewma_vol(&FIVE, 60.0, 261.0).expect("five returns give an estimate");
    assert!((v - 0.2098629870).abs() < 1e-9, "got {v}");
}
