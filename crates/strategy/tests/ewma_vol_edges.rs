//! R-141: volatility is undefined for fewer than two returns and zero for constant returns.

use pt_strategy::vol::ewma_vol;

#[test]
fn empty_returns_have_no_volatility() {
    // R-141
    assert_eq!(ewma_vol(&[], 60.0, 261.0), None);
}

#[test]
fn single_return_has_no_volatility() {
    // R-141
    assert_eq!(ewma_vol(&[0.01], 60.0, 261.0), None);
}

#[test]
fn constant_returns_have_zero_volatility() {
    // R-141
    let v = ewma_vol(&[0.01, 0.01, 0.01], 60.0, 261.0).expect("three returns give an estimate");
    assert!(v.abs() < 1e-12, "got {v}");
}
