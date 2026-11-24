use pt_stats::perf::{cagr, max_drawdown};

const EPS: f64 = 1e-9;

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "{a} != {b}");
}

#[test]
fn max_drawdown_is_largest_peak_to_trough_fall() {
    // R-233
    close(max_drawdown(&[100.0, 120.0, 90.0, 110.0, 80.0]), 1.0 / 3.0);
}

#[test]
fn max_drawdown_is_zero_for_non_decreasing_series() {
    // R-233
    close(max_drawdown(&[100.0, 100.0, 110.0, 130.0]), 0.0);
}

#[test]
fn max_drawdown_picks_the_deeper_of_two_falls() {
    // R-233: falls of 10% then 50%
    close(max_drawdown(&[100.0, 90.0, 120.0, 60.0, 70.0]), 0.5);
}

#[test]
fn cagr_compounds_growth_over_the_elapsed_years() {
    // R-233: two periods at one period per year
    close(cagr(&[100.0, 110.0, 121.0], 1.0), 0.10);
}

#[test]
fn cagr_scales_by_periods_per_year() {
    // R-233: two periods at four per year is half a year
    close(cagr(&[100.0, 110.0, 121.0], 4.0), 1.21_f64.powi(2) - 1.0);
}
