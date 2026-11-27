//! R-388: a research backtest runs from the command line, offline, and reports its
//! metrics, the Deflated Sharpe Ratio and the trial count.

mod common;

use common::offline::{env, run};
use common::{stderr, stdout};
use std::process::Output;

/// Lowercased stdout with `=` and `:` turned into spaces, split into tokens.
fn tokens(out: &Output) -> Vec<String> {
    stdout(out)
        .to_lowercase()
        .replace(['=', ':'], " ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn has_pair(toks: &[String], key: &str, value: &str) -> bool {
    toks.windows(2).any(|w| w[0] == key && w[1] == value)
}

fn trial_hash(toks: &[String]) -> Option<String> {
    toks.iter()
        .find(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))
        .cloned()
}

#[test]
fn backtest_run_exits_zero_and_prints_metrics_dsr_and_one_trial() {
    // R-388
    let e = env();
    let out = run(
        &e,
        &[
            "backtest",
            "run",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    for label in [
        "run", "sharpe", "cagr", "vol", "drawdown", "turnover", "cost", "borrow", "dsr",
    ] {
        assert!(text.contains(label), "missing {label:?} in: {text}");
    }
    let toks = tokens(&out);
    assert!(
        trial_hash(&toks).is_some(),
        "no 64-hex trial hash in: {text}"
    );
    assert!(has_pair(&toks, "trials", "1"), "no `trials 1` in: {text}");
}

#[test]
fn backtest_run_prints_the_first_run_id() {
    // R-388
    let e = env();
    let out = run(
        &e,
        &[
            "backtest",
            "run",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let toks = tokens(&out);
    assert!(
        has_pair(&toks, "run", "1") || has_pair(&toks, "id", "1"),
        "no run id 1 in: {}",
        stdout(&out)
    );
}

#[test]
fn benchmark_strategy_runs_and_exits_zero() {
    // R-388
    let e = env();
    let out = run(
        &e,
        &[
            "backtest",
            "run",
            "--strategy",
            "benchmark",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    assert!(text.contains("sharpe"), "stdout: {text}");
    assert!(has_pair(&tokens(&out), "trials", "1"), "stdout: {text}");
}

#[test]
fn second_identical_run_prints_the_same_hash_and_still_one_trial() {
    // R-388
    let e = env();
    let args = [
        "backtest",
        "run",
        "--from",
        "2024-03-01",
        "--to",
        "2024-12-30",
    ];
    let first = run(&e, &args);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    let second = run(&e, &args);
    assert_eq!(second.status.code(), Some(0), "stderr: {}", stderr(&second));
    let (t1, t2) = (tokens(&first), tokens(&second));
    assert!(trial_hash(&t1).is_some(), "stdout: {}", stdout(&first));
    assert_eq!(trial_hash(&t1), trial_hash(&t2));
    assert!(
        has_pair(&t2, "trials", "1"),
        "no `trials 1` in: {}",
        stdout(&second)
    );
}
