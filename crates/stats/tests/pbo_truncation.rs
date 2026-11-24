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

// R-241: two extra rows placed before the golden matrix are dropped from the start.
#[test]
fn two_leading_extra_rows_are_dropped() {
    let base = pbo_cscv(&golden(), 4).expect("golden matrix is valid");
    let mut padded = vec![vec![0.5, -0.5, 0.25, -0.25], vec![-0.4, 0.3, 0.2, 0.1]];
    padded.extend(golden());
    let r = pbo_cscv(&padded, 4).expect("padded matrix is valid");
    assert_eq!(r.rows_used, 12);
    assert_eq!(r.n_combinations, base.n_combinations);
    assert_eq!(r.logits.len(), base.logits.len());
    for (i, (got, want)) in r.logits.iter().zip(&base.logits).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "logit {i}: got {got}, want {want}"
        );
    }
    assert!((r.pbo - base.pbo).abs() < 1e-12);
}

// R-241: a single extra row (a remainder of one) is also dropped from the start.
#[test]
fn one_leading_extra_row_is_dropped() {
    let base = pbo_cscv(&golden(), 4).expect("golden matrix is valid");
    let mut padded = vec![vec![9.0, -9.0, 9.0, -9.0]];
    padded.extend(golden());
    let r = pbo_cscv(&padded, 4).expect("padded matrix is valid");
    assert_eq!(r.rows_used, 12);
    assert_eq!(r.logits, base.logits);
}

// R-241: when the rows already fill the partitions, nothing is dropped.
#[test]
fn exact_multiple_uses_every_row() {
    let r = pbo_cscv(&golden(), 4).expect("golden matrix is valid");
    assert_eq!(r.rows_used, 12);
}
