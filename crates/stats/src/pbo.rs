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

/// Per-block, per-trial sums and centred sums of squares, laid out `block * cols + col`.
struct BlockStats {
    cols: usize,
    block_len: f64,
    sum: Vec<f64>,
    m2: Vec<f64>,
}

impl BlockStats {
    fn new(used: &[Vec<f64>], partitions: usize, block: usize, cols: usize) -> Self {
        let mut sum = vec![0.0; partitions * cols];
        let mut m2 = vec![0.0; partitions * cols];
        for b in 0..partitions {
            let rows = &used[b * block..(b + 1) * block];
            for c in 0..cols {
                let s: f64 = rows.iter().map(|r| r[c]).sum();
                let mean = s / block as f64;
                sum[b * cols + c] = s;
                m2[b * cols + c] = rows.iter().map(|r| (r[c] - mean).powi(2)).sum();
            }
        }
        BlockStats {
            cols,
            block_len: block as f64,
            sum,
            m2,
        }
    }

    /// Sharpe ratio of trial `col` over the blocks where `mask[b] == in_sample`.
    ///
    /// Block moments are pooled exactly: the total centred sum of squares is the sum of the
    /// block ones plus each block's size times its squared offset from the pooled mean.
    fn sharpe(&self, mask: &[bool], in_sample: bool, col: usize) -> f64 {
        let mut total = 0.0;
        let mut count = 0usize;
        for (b, &m) in mask.iter().enumerate() {
            if m == in_sample {
                total += self.sum[b * self.cols + col];
                count += 1;
            }
        }
        let n = count as f64 * self.block_len;
        let mean = total / n;
        let mut ss = 0.0;
        for (b, &m) in mask.iter().enumerate() {
            if m == in_sample {
                let i = b * self.cols + col;
                let d = self.sum[i] / self.block_len - mean;
                ss += self.m2[i] + self.block_len * d * d;
            }
        }
        let sd = (ss / (n - 1.0)).sqrt();
        if sd > 0.0 {
            mean / sd
        } else {
            0.0
        }
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
    let stats = BlockStats::new(used, partitions, block, cols);
    let mut idx: Vec<usize> = (0..half).collect();
    let mut mask = vec![false; partitions];
    let mut oos = vec![0.0; cols];
    let mut logits = Vec::with_capacity(usize::try_from(n_combinations(partitions)).unwrap_or(0));
    loop {
        mask.iter_mut().for_each(|m| *m = false);
        for &b in &idx {
            mask[b] = true;
        }
        let mut best = 0;
        let mut best_sr = f64::NEG_INFINITY;
        for c in 0..cols {
            let sr = stats.sharpe(&mask, true, c);
            if sr > best_sr {
                best_sr = sr;
                best = c;
            }
        }
        for (c, v) in oos.iter_mut().enumerate() {
            *v = stats.sharpe(&mask, false, c);
        }
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
