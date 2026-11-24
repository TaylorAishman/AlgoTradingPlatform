//! Normal distribution functions.

use std::f64::consts::{FRAC_2_SQRT_PI, PI, SQRT_2, TAU};

use crate::error::StatsError;

/// Complementary error function for `z >= 0`.
fn erfc_nonneg(z: f64) -> f64 {
    if z < 3.0 {
        // erf(z) = 2/sqrt(pi) * exp(-z^2) * sum 2^n z^(2n+1) / (1*3*...*(2n+1)); all terms positive.
        let z2 = z * z;
        let mut term = z;
        let mut sum = z;
        for n in 1..200 {
            term *= 2.0 * z2 / f64::from(2 * n + 1);
            sum += term;
            if term < sum * 1e-17 {
                break;
            }
        }
        1.0 - FRAC_2_SQRT_PI * (-z2).exp() * sum
    } else {
        // Continued fraction: erfc(z) = exp(-z^2)/sqrt(pi) / (z + (1/2)/(z + 1/(z + (3/2)/(z + ...)))).
        let mut frac = z;
        for k in (1..=80).rev() {
            frac = z + f64::from(k) / 2.0 / frac;
        }
        (-z * z).exp() / (PI.sqrt() * frac)
    }
}

/// Standard normal cumulative distribution function.
pub fn norm_cdf(x: f64) -> f64 {
    let tail = 0.5 * erfc_nonneg(x.abs() / SQRT_2);
    if x >= 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

/// Inverse of the standard normal CDF for `p` in the open interval (0, 1).
///
/// Uses Acklam's rational approximation for the lower tail (by symmetry for the upper),
/// then polishes with Halley steps against `norm_cdf`.
pub fn norm_inv_cdf(p: f64) -> Result<f64, StatsError> {
    if p.is_nan() || p <= 0.0 || p >= 1.0 {
        return Err(StatsError::OutOfDomain(format!(
            "probability must be in (0, 1), got {p}"
        )));
    }
    if p == 0.5 {
        return Ok(0.0);
    }
    let q = p.min(1.0 - p);
    let mut x = acklam_lower(q);
    for _ in 0..3 {
        let err = norm_cdf(x) - q;
        let u = err * TAU.sqrt() * (x * x / 2.0).exp();
        x -= u / (1.0 + x * u / 2.0);
    }
    Ok(if p < 0.5 { x } else { -x })
}

/// Acklam's approximation for `q` in (0, 0.5); relative error about 1e-9.
fn acklam_lower(q: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383_577_518_672_69e2,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;
    if q < P_LOW {
        let t = (-2.0 * q.ln()).sqrt();
        (((((C[0] * t + C[1]) * t + C[2]) * t + C[3]) * t + C[4]) * t + C[5])
            / ((((D[0] * t + D[1]) * t + D[2]) * t + D[3]) * t + 1.0)
    } else {
        let r = (q - 0.5) * (q - 0.5);
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * (q - 0.5)
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}
