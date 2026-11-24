use pt_stats::error::StatsError;
use pt_stats::ols::ols;

const X: [f64; 5] = [0.01, -0.02, 0.03, 0.00, 0.015];
const Y: [f64; 5] = [0.012, -0.015, 0.031, 0.004, 0.010];

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn regression_reports_intercept_slope_t_statistic_and_fit() {
    // R-245
    let r = ols(&Y, &X).unwrap();
    close(r.alpha, 0.0022826087);
    close(r.beta, 0.8739130435);
    close(r.alpha_t, 1.3001995168);
    close(r.r2, 0.9640862884);
    assert_eq!(r.n, 5);
}

#[test]
fn fewer_than_three_points_is_too_few_observations() {
    // R-245
    assert_eq!(
        ols(&[1.0, 2.0], &[1.0, 2.0]).unwrap_err(),
        StatsError::TooFewObservations
    );
    assert_eq!(ols(&[], &[]).unwrap_err(), StatsError::TooFewObservations);
}

#[test]
fn constant_x_is_too_few_observations() {
    // R-245
    assert_eq!(
        ols(&[1.0, 2.0, 4.0, 3.0], &[0.5, 0.5, 0.5, 0.5]).unwrap_err(),
        StatsError::TooFewObservations
    );
}
