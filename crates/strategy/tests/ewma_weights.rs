use pt_strategy::vol::{ewma_cov, ewma_vol, ewma_weights};

// R-450: weights are normalised and newest first.
#[test]
fn three_weights_at_com_one_are_four_two_one_sevenths() {
    let w = ewma_weights(3, 1.0);
    let want = [4.0 / 7.0, 2.0 / 7.0, 1.0 / 7.0];
    assert_eq!(w.len(), 3);
    for (a, b) in w.iter().zip(want) {
        assert!((a - b).abs() < 1e-15, "{a} vs {b}");
    }
}

// R-450
#[test]
fn zero_weights_is_empty() {
    assert!(ewma_weights(0, 1.0).is_empty());
}

// R-450
#[test]
fn sixty_weights_sum_to_one_and_decay_geometrically() {
    let w = ewma_weights(60, 60.0);
    assert_eq!(w.len(), 60);
    assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    for pair in w.windows(2) {
        assert!((pair[1] - pair[0] * 60.0 / 61.0).abs() < 1e-12);
    }
}

// R-450
#[test]
fn cov_of_a_series_with_itself_is_vol_squared() {
    let a = [0.01, -0.02, 0.015, 0.003, -0.007];
    let v = ewma_vol(&a, 60.0, 261.0).unwrap();
    let c = ewma_cov(&a, &a, 60.0, 261.0).unwrap();
    assert!((c - v * v).abs() < 1e-15);
}
