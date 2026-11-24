use pt_stats::descriptive::{mean, returns_from_equity, std_sample};

const EPS: f64 = 1e-9;

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "{a} != {b}");
}

#[test]
fn mean_of_four_returns() {
    // R-230
    close(mean(&[0.01, 0.02, -0.01, 0.03]), 0.0125);
}

#[test]
fn std_sample_of_four_returns() {
    // R-230
    close(std_sample(&[0.01, 0.02, -0.01, 0.03]), 0.0170782513);
}

#[test]
fn std_sample_of_one_value_is_zero() {
    // R-230
    assert_eq!(std_sample(&[0.05]), 0.0);
}

#[test]
fn returns_from_equity_gives_simple_returns() {
    // R-230
    let r = returns_from_equity(&[100.0, 110.0, 99.0]);
    assert_eq!(r.len(), 2);
    close(r[0], 0.10);
    close(r[1], -0.10);
}
