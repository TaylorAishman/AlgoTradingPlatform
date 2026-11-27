//! R-391: `validate pbo` prints the Probability of Backtest Overfitting of stored runs.

mod common;

use common::offline::{env, run, Env};
use common::{stderr, stdout};

/// Rewrites the config with another `lookback_months`, so the next run is a new trial.
fn set_lookback(e: &Env, months: u32) {
    let text = std::fs::read_to_string(&e.cfg).unwrap();
    assert!(
        text.contains("lookback_months = ["),
        "fixture config changed"
    );
    let start = text.find("lookback_months = [").unwrap();
    let end = start + text[start..].find(']').unwrap() + 1;
    let new = format!(
        "{}lookback_months = [{months}]{}",
        &text[..start],
        &text[end..]
    );
    std::fs::write(&e.cfg, new).unwrap();
}

fn backtest(e: &Env) {
    let out = run(
        e,
        &[
            "backtest",
            "run",
            "--from",
            "2024-04-01",
            "--to",
            "2024-12-30",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

fn one_run() -> Env {
    let e = env();
    backtest(&e);
    e
}

fn two_runs() -> Env {
    let e = one_run();
    set_lookback(&e, 2);
    backtest(&e);
    e
}

#[test]
fn validate_pbo_prints_pbo_and_the_number_of_combinations() {
    // R-391: default pbo_partitions = 16, so C(16, 8) = 12870 combinations
    let e = two_runs();
    let out = run(&e, &["validate", "pbo", "--runs", "1,2"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    assert!(text.contains("pbo"), "missing pbo in: {text}");
    assert!(
        text.contains("combinations"),
        "missing combinations in: {text}"
    );
    assert!(
        text.replace([',', '_'], "").contains("12870"),
        "missing 12870 in: {text}"
    );
}

#[test]
fn validate_pbo_with_one_run_exits_one_and_says_it_is_undefined() {
    // R-391 (error case: fewer than 2 trials)
    let e = one_run();
    let out = run(&e, &["validate", "pbo", "--runs", "1"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    let all = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        all.contains("PBO is undefined for fewer than 2 trials"),
        "output: {all}"
    );
}
