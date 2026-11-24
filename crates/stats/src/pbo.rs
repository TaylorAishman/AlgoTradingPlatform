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
    /// Number of combinations evaluated.
    pub n_combinations: usize,
    /// Number of rows used after trimming to a multiple of the partition count.
    pub rows_used: usize,
}

fn sharpe(rows: &[&Vec<f64>], col: usize) -> f64 {
    let n = rows.len() as f64;
    let mean = rows.iter().map(|r| r[col]).sum::<f64>() / n;
    let var = rows.iter().map(|r| (r[col] - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = var.sqrt();
    if sd > 0.0 {
        mean / sd
    } else {
        0.0
    }
}

/// Next combination of `k` indices out of `n` in lexicographic order; false when exhausted.
fn next_combination(idx: &mut [usize], n: usize) -> bool {
    let k = idx.len();
    let mut i = k;
    while i > 0 {
        i -= 1;
        if idx[i] != i + n - k {
            idx[i] += 1;
            for j in i + 1..k {
                idx[j] = idx[j - 1] + 1;
            }
            return true;
        }
    }
    false
}

/// Probability of backtest overfitting over a returns matrix (rows are periods, columns are trials).
///
/// Returns the logits and PBO over every combination of partitions; a zero-std Sharpe counts as 0.0.
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
    let block = returns.len() / partitions;
    let rows_used = block * partitions;
    let used = &returns[returns.len() - rows_used..];
    let half = partitions / 2;
    let mut idx: Vec<usize> = (0..half).collect();
    let mut logits = Vec::new();
    loop {
        let mut is_rows: Vec<&Vec<f64>> = Vec::new();
        let mut oos_rows: Vec<&Vec<f64>> = Vec::new();
        for b in 0..partitions {
            let target = if idx.contains(&b) {
                &mut is_rows
            } else {
                &mut oos_rows
            };
            target.extend(used[b * block..(b + 1) * block].iter());
        }
        let mut best = 0;
        let mut best_sr = f64::NEG_INFINITY;
        for c in 0..cols {
            let sr = sharpe(&is_rows, c);
            if sr > best_sr {
                best_sr = sr;
                best = c;
            }
        }
        let oos: Vec<f64> = (0..cols).map(|c| sharpe(&oos_rows, c)).collect();
        // Ascending rank, 1 for the worst, mean rank on ties (architecture 5.10.7 step 4).
        let less = oos.iter().filter(|&&v| v < oos[best]).count();
        let equal = oos.iter().filter(|&&v| v == oos[best]).count();
        let rank = 1.0 + less as f64 + (equal as f64 - 1.0) / 2.0;
        let omega = rank / (cols as f64 + 1.0);
        logits.push((omega / (1.0 - omega)).ln());
        if !next_combination(&mut idx, partitions) {
            break;
        }
    }
    let pbo = logits.iter().filter(|&&l| l <= 0.0).count() as f64 / logits.len() as f64;
    Ok(PboResult {
        pbo,
        n_combinations: logits.len(),
        logits,
        rows_used,
    })
}
