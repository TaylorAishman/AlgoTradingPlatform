use pt_stats::pbo::n_combinations;

// R-244: the number of combinations is the binomial coefficient C(s, s/2).
#[test]
fn four_partitions_give_six_combinations() {
    assert_eq!(n_combinations(4), 6);
}

// R-244: the default partition count gives 12870 combinations.
#[test]
fn sixteen_partitions_give_12870_combinations() {
    assert_eq!(n_combinations(16), 12870);
}

// R-244: other even partition counts follow the same binomial coefficient.
#[test]
fn other_even_partition_counts_follow_the_binomial_coefficient() {
    assert_eq!(n_combinations(2), 2);
    assert_eq!(n_combinations(6), 20);
    assert_eq!(n_combinations(8), 70);
    assert_eq!(n_combinations(10), 252);
}
