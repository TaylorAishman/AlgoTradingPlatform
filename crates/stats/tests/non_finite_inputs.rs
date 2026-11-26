//! R-464: a statistic that reports errors refuses an input that is not a finite number.

use pt_stats::dsr::{deflated_sharpe, psr};
use pt_stats::error::StatsError;
use pt_stats::normal::norm_inv_cdf;
use pt_stats::ols::ols;
use pt_stats::pbo::pbo_cscv;

fn is_out_of_domain<T: std::fmt::Debug>(r: Result<T, StatsError>) -> bool {
    matches!(r, Err(StatsError::OutOfDomain(_)))
}

// R-464: ols with a NaN in y returns OutOfDomain.
#[test]
fn ols_with_nan_in_y_is_out_of_domain() {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.1, f64::NAN, 2.9, 4.2, 4.8];
    assert!(is_out_of_domain(ols(&y, &x)));
}

// R-464: ols with an infinity in x returns OutOfDomain.
#[test]
fn ols_with_infinity_in_x_is_out_of_domain() {
    let x = [1.0, 2.0, f64::INFINITY, 4.0, 5.0];
    let y = [1.1, 2.1, 2.9, 4.2, 4.8];
    assert!(is_out_of_domain(ols(&y, &x)));
}

// R-464: psr with sr = NaN returns OutOfDomain.
#[test]
fn psr_with_nan_sharpe_is_out_of_domain() {
    assert!(is_out_of_domain(psr(f64::NAN, 100, 0.0, 3.0, 0.0)));
}

// R-464: deflated_sharpe with sr = NaN returns OutOfDomain.
#[test]
fn deflated_sharpe_with_nan_sharpe_is_out_of_domain() {
    assert!(is_out_of_domain(deflated_sharpe(
        f64::NAN,
        100,
        0.0,
        3.0,
        0.002,
        10
    )));
}

// R-464: pbo_cscv on a matrix holding one NaN returns OutOfDomain.
#[test]
fn pbo_cscv_with_one_nan_is_out_of_domain() {
    let mut m: Vec<Vec<f64>> = (0..12)
        .map(|i| {
            (0..4)
                .map(|j| ((i * 7 + j * 3) % 11) as f64 * 0.001 - 0.004)
                .collect()
        })
        .collect();
    m[5][2] = f64::NAN;
    assert!(is_out_of_domain(pbo_cscv(&m, 4)));
}

// R-464: norm_inv_cdf(NaN) returns OutOfDomain.
#[test]
fn norm_inv_cdf_of_nan_is_out_of_domain() {
    assert!(is_out_of_domain(norm_inv_cdf(f64::NAN)));
}
