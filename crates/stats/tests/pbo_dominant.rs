use pt_stats::pbo::pbo_cscv;

fn dominant() -> Vec<Vec<f64>> {
    (0..16)
        .map(|i: usize| {
            vec![
                0.01 + 0.001 * (i % 3) as f64,
                0.002 * (((7 * i) % 5) as f64 - 2.0),
                -0.01 + 0.001 * ((3 * i) % 4) as f64,
            ]
        })
        .collect()
}

// R-243: a trial that wins in every sample gives a PBO of zero.
#[test]
fn dominant_trial_gives_zero_pbo() {
    let r = pbo_cscv(&dominant(), 4).expect("matrix is valid");
    assert_eq!(r.pbo, 0.0);
}

// R-243: every logit is ln(3) because the winner ranks first out of three trials.
#[test]
fn dominant_trial_logits_are_ln_three() {
    let r = pbo_cscv(&dominant(), 4).expect("matrix is valid");
    assert_eq!(r.logits.len(), 6);
    for (i, l) in r.logits.iter().enumerate() {
        assert!((l - 1.098612).abs() < 1e-6, "logit {i}: got {l}");
    }
}
