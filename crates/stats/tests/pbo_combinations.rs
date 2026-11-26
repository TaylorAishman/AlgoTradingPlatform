use pt_stats::error::StatsError;
use pt_stats::pbo::{n_combinations, pbo_cscv};

// R-244: the number of combinations is the binomial coefficient C(s, s/2).
#[test]
fn four_partitions_give_six_combinations() {
    assert_eq!(n_combinations(4), Ok(6));
}

// R-244: the default partition count gives 12870 combinations.
#[test]
fn sixteen_partitions_give_12870_combinations() {
    assert_eq!(n_combinations(16), Ok(12870));
}

// R-244, R-463: other even partition counts follow the same binomial coefficient.
#[test]
fn other_even_partition_counts_follow_the_binomial_coefficient() {
    assert_eq!(n_combinations(2), Ok(2));
    assert_eq!(n_combinations(6), Ok(20));
    assert_eq!(n_combinations(8), Ok(70));
    assert_eq!(n_combinations(10), Ok(252));
}

// R-462: the largest count that fits in a u64 is still returned.
#[test]
fn sixty_six_partitions_give_the_largest_count_that_fits() {
    assert_eq!(n_combinations(66), Ok(7_219_428_434_016_265_740));
}

// R-462: a count too large to hold is an out-of-domain error.
#[test]
fn sixty_eight_partitions_overflow_to_out_of_domain() {
    assert!(matches!(
        n_combinations(68),
        Err(StatsError::OutOfDomain(_))
    ));
}

// R-463: zero and odd partition counts are invalid partitions.
#[test]
fn zero_and_odd_partition_counts_are_invalid() {
    for s in [0usize, 3, 7] {
        assert!(
            matches!(n_combinations(s), Err(StatsError::InvalidPartition(_))),
            "s = {s}"
        );
    }
}

// R-463: pbo_cscv with an odd partition count is an invalid partition.
#[test]
fn pbo_cscv_with_three_partitions_is_invalid() {
    let m: Vec<Vec<f64>> = (0..30)
        .map(|i| vec![i as f64 * 0.001, (i % 5) as f64 * 0.002])
        .collect();
    assert!(matches!(
        pbo_cscv(&m, 3),
        Err(StatsError::InvalidPartition(_))
    ));
}
