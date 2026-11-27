//! R-549: the validation commands print fixed `label value` lines.

mod common;

use common::offline::{env, run, Env};
use common::{stderr, stdout};

fn backtest(e: &Env, strategy: &str) {
    let out = run(
        e,
        &[
            "backtest",
            "run",
            "--strategy",
            strategy,
            "--from",
            "2024-04-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

/// Two stored runs: tsmom (1) and benchmark (2).
fn two_runs() -> Env {
    let e = env();
    backtest(&e, "tsmom");
    backtest(&e, "benchmark");
    e
}

/// Asserts every stdout line is `label value` with exactly the given labels in order, and
/// that every value outside `integers` parses as a finite decimal number.
fn assert_labels(args: &[&str], e: &Env, labels: &[&str], integers: &[&str]) {
    let out = run(e, args);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    let mut seen = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        assert_eq!(parts.len(), 2, "not `label value`: {line:?} in: {text}");
        let (label, value) = (parts[0], parts[1]);
        if integers.contains(&label) {
            assert!(
                value.parse::<u64>().is_ok(),
                "{label} is not an integer: {line:?}"
            );
        } else {
            let n: f64 = value
                .parse()
                .unwrap_or_else(|_| panic!("{label} is not a number: {line:?}"));
            assert!(n.is_finite(), "{label} is not finite: {line:?}");
        }
        seen.push(label.to_owned());
    }
    assert_eq!(seen, labels, "labels in: {text}");
}

#[test]
fn validate_dsr_prints_exactly_the_labels_dsr_sr0_skew_kurt_trials() {
    // R-549
    let e = two_runs();
    assert_labels(
        &["validate", "dsr", "--run", "1"],
        &e,
        &["dsr", "sr0", "skew", "kurt", "trials"],
        &["trials"],
    );
}

#[test]
fn validate_pbo_prints_exactly_the_labels_pbo_combinations_rows() {
    // R-549
    let e = env();
    backtest(&e, "tsmom");
    backtest(&e, "benchmark");
    assert_labels(
        &["validate", "pbo", "--runs", "1,2"],
        &e,
        &["pbo", "combinations", "rows"],
        &["combinations", "rows"],
    );
}

#[test]
fn validate_benchmark_prints_exactly_the_five_comparison_labels() {
    // R-549
    let e = two_runs();
    assert_labels(
        &[
            "validate",
            "benchmark",
            "--run",
            "1",
            "--benchmark-run",
            "2",
        ],
        &e,
        &[
            "strategy_sharpe",
            "benchmark_sharpe",
            "alpha",
            "beta",
            "alpha_t_stat",
        ],
        &[],
    );
}
