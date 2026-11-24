use pt_stats::pbo::pbo_cscv;

fn golden() -> Vec<Vec<f64>> {
    vec![
        vec![0.011, -0.011, 0.02, -0.024],
        vec![-0.021, 0.038, -0.018, 0.016],
        vec![-0.023, 0.034, -0.003, -0.026],
        vec![-0.019, 0.025, 0.023, -0.022],
        vec![0.0, -0.019, 0.04, 0.024],
        vec![-0.023, -0.015, -0.002, -0.023],
        vec![0.02, -0.024, -0.002, -0.025],
        vec![-0.013, 0.007, 0.023, -0.012],
        vec![0.039, -0.015, 0.009, -0.007],
        vec![-0.017, -0.006, 0.017, -0.018],
        vec![0.04, -0.022, -0.023, -0.004],
        vec![0.033, 0.038, 0.024, 0.01],
    ]
}

// R-240: the golden 12 x 4 matrix with s = 4 reports six combinations over twelve rows.
#[test]
fn golden_matrix_reports_combination_and_row_counts() {
    let r = pbo_cscv(&golden(), 4).expect("golden matrix is valid");
    assert_eq!(r.n_combinations, 6);
    assert_eq!(r.rows_used, 12);
}

// R-240: logits match the published values, in lexicographic combination order.
#[test]
fn golden_matrix_logits_match_in_order() {
    let r = pbo_cscv(&golden(), 4).expect("golden matrix is valid");
    let expected = [0.405465, 1.386294, -0.405465, 0.405465, 1.386294, -1.386294];
    assert_eq!(r.logits.len(), expected.len());
    for (i, (got, want)) in r.logits.iter().zip(expected).enumerate() {
        assert!(
            (got - want).abs() < 1e-6,
            "logit {i}: got {got}, want {want}"
        );
    }
}

// R-240: PBO is the share of combinations with lambda <= 0 (2 of 6).
#[test]
fn golden_matrix_pbo_is_one_third() {
    let r = pbo_cscv(&golden(), 4).expect("golden matrix is valid");
    assert!((r.pbo - 0.3333333333).abs() < 1e-9, "{}", r.pbo);
}

/// Eight rows of `[v, v, -v]`: trials 0 and 1 are identical, so they always tie.
fn tied() -> Vec<Vec<f64>> {
    [0.03, 0.01, 0.02, 0.06, -0.01, -0.02, 0.01, -0.06]
        .iter()
        .map(|&v| vec![v, v, -v])
        .collect()
}

// R-240, architecture 5.10.7 step 4: a winner that ties out of sample gets the mean rank.
// When trial 0 wins in sample, it shares ranks 1-2 (omega 1.5/4) or 2-3 (omega 2.5/4) with
// trial 1. When trial 2 wins, it is alone at rank 1 (omega 1/4).
#[test]
fn tied_out_of_sample_values_take_the_mean_rank() {
    let r = pbo_cscv(&tied(), 4).expect("tied matrix is valid");
    let low = (0.375f64 / 0.625).ln();
    let high = (0.625f64 / 0.375).ln();
    let alone = (0.25f64 / 0.75).ln();
    let expected = [low, high, alone, low, high, alone];
    assert_eq!(r.logits.len(), expected.len());
    for (i, (got, want)) in r.logits.iter().zip(expected).enumerate() {
        assert!(
            (got - want).abs() < 1e-6,
            "logit {i}: got {got}, want {want}"
        );
    }
    assert!((r.pbo - 4.0 / 6.0).abs() < 1e-12, "{}", r.pbo);
}
