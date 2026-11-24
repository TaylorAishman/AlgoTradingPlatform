//! Normal distribution functions.

use std::f64::consts::{FRAC_2_SQRT_PI, PI, SQRT_2};

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
