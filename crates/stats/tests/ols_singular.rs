//! R-466: a regression whose result would not be finite is an error.

use pt_stats::error::StatsError;
use pt_stats::ols::ols;

// R-466: a perfect fit (zero residuals) would give an infinite t-statistic.
#[test]
fn perfect_fit_is_out_of_domain() {
    let x = [1.0, 2.0, 3.0, 4.0];
    let y = [3.0, 5.0, 7.0, 9.0];
    assert!(matches!(ols(&y, &x), Err(StatsError::OutOfDomain(_))));
}

// R-466: a constant x still reports too few observations.
#[test]
fn constant_x_is_too_few_observations() {
    let x = [2.0, 2.0, 2.0, 2.0];
    let y = [3.0, 5.0, 7.0, 9.0];
    assert_eq!(ols(&y, &x), Err(StatsError::TooFewObservations));
}

// R-466: with noise every field of the result is finite.
#[test]
fn noisy_data_gives_finite_fields() {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.1, 2.3, 2.8, 4.2, 4.9];
    let r = ols(&y, &x).expect("noisy data regresses");
    assert!(r.alpha.is_finite());
    assert!(r.beta.is_finite());
    assert!(r.alpha_t.is_finite());
    assert!(r.r2.is_finite());
    assert_eq!(r.n, 5);
}
