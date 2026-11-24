use std::time::{Duration, Instant};

use pt_stats::pbo::{n_combinations, pbo_cscv};

/// Deterministic pseudo-random matrix (linear congruential generator), rows x cols.
fn seeded(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    ((state >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 0.04
                })
                .collect()
        })
        .collect()
}

// R-244: the combination count is the binomial coefficient.
#[test]
fn combination_counts_are_binomial() {
    assert_eq!(n_combinations(4), 6);
    assert_eq!(n_combinations(16), 12870);
}

// R-244: s = 16 on a seeded 2520 x 20 matrix returns 12870 logits in under 10 seconds.
#[test]
fn sixteen_partitions_on_2520_by_20_finish_in_ten_seconds() {
    let m = seeded(2520, 20);
    let start = Instant::now();
    let r = pbo_cscv(&m, 16).expect("matrix is valid");
    let elapsed = start.elapsed();
    assert_eq!(r.logits.len(), 12870);
    assert_eq!(r.n_combinations, 12870);
    assert_eq!(r.rows_used, 2512); // 2520 trimmed to a multiple of 16
    assert!(r.logits.iter().all(|l| l.is_finite()));
    assert!((0.0..=1.0).contains(&r.pbo));
    assert!(
        elapsed < Duration::from_secs(10),
        "took {elapsed:?}, limit 10s"
    );
}
