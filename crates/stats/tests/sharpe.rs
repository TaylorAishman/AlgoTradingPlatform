use pt_stats::perf::{annualized_sharpe, annualized_vol, sharpe};

const EPS: f64 = 1e-9;
const R: [f64; 4] = [0.01, 0.02, -0.01, 0.03];

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "{a} != {b}");
}

#[test]
fn sharpe_is_mean_over_sample_std() {
    // R-232
    close(sharpe(&R), 0.7319250547);
}

#[test]
fn annualized_sharpe_scales_by_sqrt_periods() {
    // R-232
    close(annualized_sharpe(&R, 252.0), 11.6189500386);
}

#[test]
fn annualized_vol_scales_std_by_sqrt_periods() {
    // R-232
    close(annualized_vol(&R, 252.0), 0.2711088342);
}

#[test]
fn sharpe_is_zero_without_variation() {
    // R-232
    assert_eq!(sharpe(&[0.01, 0.01]), 0.0);
}
