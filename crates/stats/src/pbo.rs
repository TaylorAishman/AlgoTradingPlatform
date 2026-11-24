//! Probability of Backtest Overfitting via combinatorially symmetric cross-validation.

use crate::error::StatsError;

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

/// Result of a CSCV run.
#[derive(Debug, Clone, PartialEq)]
pub struct PboResult {
    /// Probability of backtest overfitting.
    pub pbo: f64,
    /// Logit of the relative rank of the in-sample winner, one per combination.
    pub logits: Vec<f64>,
}

/// Probability of backtest overfitting over a returns matrix (rows are periods, columns are trials).
///
/// Currently validates input only; the computation arrives in a later step.
pub fn pbo_cscv(returns: &[Vec<f64>], partitions: usize) -> Result<PboResult, StatsError> {
    if partitions == 0 || !partitions.is_multiple_of(2) {
        return Err(StatsError::InvalidPartition(format!(
            "partition count must be a positive even number, got {partitions}"
        )));
    }
    let cols = returns.first().map_or(0, Vec::len);
    if returns.iter().any(|r| r.len() != cols) {
        return Err(StatsError::InvalidPartition(
            "rows have unequal length".to_string(),
        ));
    }
    if cols < 2 || returns.len() < 2 * partitions {
        return Err(StatsError::TooFewObservations);
    }
    Err(StatsError::OutOfDomain(
        "pbo computation not yet implemented".to_string(),
    ))
}
