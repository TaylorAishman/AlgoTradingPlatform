//! R-392: `validate benchmark` compares a stored run with a stored benchmark run.

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

fn two_runs() -> Env {
    let e = env();
    backtest(&e, "tsmom");
    backtest(&e, "benchmark");
    e
}

#[test]
fn validate_benchmark_prints_both_sharpes_alpha_beta_and_alpha_t_statistic() {
    // R-392
    let e = two_runs();
    let out = run(
        &e,
        &[
            "validate",
            "benchmark",
            "--run",
            "1",
            "--benchmark-run",
            "2",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    for label in ["alpha", "beta"] {
        assert!(text.contains(label), "missing {label:?} in: {text}");
    }
    assert!(
        text.contains("t-stat") || text.contains("t_stat") || text.contains("tstat"),
        "missing t-statistic in: {text}"
    );
    let sharpes = text.matches("sharpe").count();
    assert!(
        sharpes >= 2,
        "want two Sharpe ratios, got {sharpes} in: {text}"
    );
}

#[test]
fn validate_benchmark_for_an_unknown_run_exits_one() {
    // R-392 (error case: no such run)
    let e = two_runs();
    let out = run(
        &e,
        &[
            "validate",
            "benchmark",
            "--run",
            "99",
            "--benchmark-run",
            "2",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    assert!(stderr(&out).contains("99"), "stderr: {}", stderr(&out));
}
