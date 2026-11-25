//! R-336: slippage is positive when a fill is worse than the reference price.

use std::str::FromStr;

use pt_core::types::Side;
use pt_shadow::slippage::slippage_bps;
use rust_decimal::Decimal;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

// R-336
#[test]
fn buy_filled_above_reference_is_positive() {
    close(slippage_bps(Side::Buy, d("100.05"), d("100")), 5.0);
}

// R-336
#[test]
fn sell_filled_below_reference_is_positive() {
    close(slippage_bps(Side::Sell, d("99.95"), d("100")), 5.0);
}

// R-336
#[test]
fn buy_filled_below_reference_is_negative() {
    close(slippage_bps(Side::Buy, d("99.90"), d("100")), -10.0);
}

// R-336
#[test]
fn sell_filled_above_reference_is_negative() {
    close(slippage_bps(Side::Sell, d("100.10"), d("100")), -10.0);
}
