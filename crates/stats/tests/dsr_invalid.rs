//! R-239: the Deflated Sharpe Ratio refuses inputs it cannot evaluate.

use pt_stats::dsr::{deflated_sharpe, psr};
use pt_stats::error::StatsError;

// R-239: t of 1 returns TooFewObservations.
#[test]
fn psr_with_one_observation_is_too_few() {
    assert_eq!(
        psr(0.1, 1, 0.0, 3.0, 0.0),
        Err(StatsError::TooFewObservations)
    );
}

// R-239: t of 1 returns TooFewObservations.
#[test]
fn deflated_sharpe_with_one_observation_is_too_few() {
    assert_eq!(
        deflated_sharpe(0.1, 1, 0.0, 3.0, 0.002, 10),
        Err(StatsError::TooFewObservations)
    );
}

// R-239: a negative variance term returns OutOfDomain (sr = 1, skew = 3, kurt = 1).
#[test]
fn psr_with_negative_variance_term_is_out_of_domain() {
    assert!(matches!(
        psr(1.0, 100, 3.0, 1.0, 0.0),
        Err(StatsError::OutOfDomain(_))
    ));
}

// R-239: a negative variance term returns OutOfDomain.
#[test]
fn deflated_sharpe_with_negative_variance_term_is_out_of_domain() {
    assert!(matches!(
        deflated_sharpe(1.0, 100, 3.0, 1.0, 0.002, 10),
        Err(StatsError::OutOfDomain(_))
    ));
}

// R-239: a zero variance term (1 - 1 * 1 + 0 = 0) returns OutOfDomain.
#[test]
fn psr_with_zero_variance_term_is_out_of_domain() {
    assert!(matches!(
        psr(1.0, 100, 1.0, 1.0, 0.0),
        Err(StatsError::OutOfDomain(_))
    ));
}

// R-239: valid inputs still evaluate.
#[test]
fn psr_with_valid_inputs_still_evaluates() {
    let p = psr(0.1, 100, 0.0, 3.0, 0.1).unwrap();
    assert!((p - 0.5).abs() < 1e-12);
}
