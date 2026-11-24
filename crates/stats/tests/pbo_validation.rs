use pt_stats::error::StatsError;
use pt_stats::pbo::pbo_cscv;

fn matrix(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|i| {
            (0..cols)
                .map(|j| 0.001 * (((i * 7 + j * 3) % 11) as f64 - 5.0))
                .collect()
        })
        .collect()
}

// R-242: an odd partition count is refused.
#[test]
fn odd_partition_count_is_invalid_partition() {
    let r = pbo_cscv(&matrix(12, 4), 3);
    assert!(matches!(r, Err(StatsError::InvalidPartition(_))), "{r:?}");
}

// R-242: zero partitions is refused.
#[test]
fn zero_partitions_is_invalid_partition() {
    let r = pbo_cscv(&matrix(12, 4), 0);
    assert!(matches!(r, Err(StatsError::InvalidPartition(_))), "{r:?}");
}

// R-242: a matrix with one column (one trial) is refused.
#[test]
fn single_column_is_too_few_observations() {
    let r = pbo_cscv(&matrix(12, 1), 4);
    assert!(matches!(r, Err(StatsError::TooFewObservations)), "{r:?}");
}

// R-242: fewer than two rows per block is refused.
#[test]
fn six_rows_with_four_partitions_is_too_few_observations() {
    let r = pbo_cscv(&matrix(6, 4), 4);
    assert!(matches!(r, Err(StatsError::TooFewObservations)), "{r:?}");
}

// R-242: rows of unequal length are refused.
#[test]
fn ragged_rows_are_invalid_partition() {
    let mut m = matrix(12, 4);
    m[5].pop();
    let r = pbo_cscv(&m, 4);
    assert!(matches!(r, Err(StatsError::InvalidPartition(_))), "{r:?}");
}
