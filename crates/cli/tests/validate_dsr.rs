//! R-391: the Deflated Sharpe Ratio of a stored run is printed by `validate dsr`.

mod common;

use common::offline::{env, run, Env};
use common::{stderr, stdout};
use std::process::Output;

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

fn two_runs() -> Env {
    let e = env();
    backtest(&e);
    set_lookback(&e, 2);
    backtest(&e);
    e
}

#[test]
fn validate_dsr_prints_dsr_sr0_trials_skewness_and_kurtosis() {
    // R-391
    let e = two_runs();
    let out = run(&e, &["validate", "dsr", "--run", "1"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out).to_lowercase();
    for label in ["dsr", "sr0", "skew", "kurt"] {
        assert!(text.contains(label), "missing {label:?} in: {text}");
    }
    assert!(
        has_pair(&tokens(&out), "trials", "2"),
        "no `trials 2` in: {text}"
    );
}

#[test]
fn validate_dsr_for_an_unknown_run_exits_one() {
    // R-391 (error case: no such run)
    let e = two_runs();
    let out = run(&e, &["validate", "dsr", "--run", "99"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    assert!(stderr(&out).contains("99"), "stderr: {}", stderr(&out));
}

fn backtest_strategy(e: &Env, strategy: &str) {
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

#[test]
fn validate_dsr_counts_only_the_trials_of_the_runs_own_family() {
    // R-548: tsmom (1), tsmom with another lookback (2), benchmark (3)
    let e = env();
    backtest_strategy(&e, "tsmom");
    set_lookback(&e, 2);
    backtest_strategy(&e, "tsmom");
    backtest_strategy(&e, "benchmark");

    let out = run(&e, &["validate", "dsr", "--run", "3"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        has_pair(&tokens(&out), "trials", "1"),
        "no `trials 1` for the benchmark run in: {}",
        stdout(&out)
    );

    let out = run(&e, &["validate", "dsr", "--run", "1"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        has_pair(&tokens(&out), "trials", "2"),
        "no `trials 2` for the tsmom run in: {}",
        stdout(&out)
    );
}
