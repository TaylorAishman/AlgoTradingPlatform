//! Probability of Backtest Overfitting via combinatorially symmetric cross-validation.

/// Number of ways to choose half of `partitions` blocks: the binomial coefficient C(s, s/2).
///
/// `partitions` is expected to be even; an odd count uses `s / 2` rounded down.
pub fn n_combinations(partitions: usize) -> u64 {
    let k = (partitions / 2) as u64;
    let n = partitions as u64;
    let mut result: u64 = 1;
    for i in 1..=k {
        // result * (n - k + i) is always divisible by i at this point.
        result = result * (n - k + i) / i;
    }
    result
}
