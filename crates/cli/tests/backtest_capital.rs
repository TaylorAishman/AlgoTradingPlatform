//! R-541: `backtest run` takes its starting capital from `--capital`.
//! R-542, R-543: without the flag it uses the configured capital; capital does not change the trial.

mod common;

use common::offline::{env, run};
use common::{stderr, stdout};
use rust_decimal::Decimal;

fn has_line(text: &str, label: &str, value: &str) -> bool {
    text.lines()
        .any(|l| l.split_whitespace().collect::<Vec<_>>() == [label, value])
}

fn store(e: &common::offline::Env) -> pt_store::db::Store {
    pt_store::db::Store::open(&e.dir.path().join("test.sqlite")).unwrap()
}

#[test]
fn capital_flag_sets_the_capital_and_is_stored_with_source_flag() {
    // R-541
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
            "--capital",
            "250000",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(has_line(&text, "capital", "250000"), "stdout: {text}");
    assert!(has_line(&text, "source", "flag"), "stdout: {text}");
    assert_eq!(
        store(&e).backtest_capital(1).unwrap(),
        Some((Decimal::new(250_000, 0), "flag".to_string()))
    );
}

#[test]
fn zero_capital_exits_one_and_stores_no_run() {
    // R-541
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
            "--capital",
            "0",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(
        matches!(
            store(&e).backtest_capital(1),
            Err(pt_store::error::StoreError::NotFound(_))
        ),
        "a run was stored"
    );
}

const RANGE: [&str; 4] = ["--from", "2024-03-01", "--to", "2024-12-30"];

fn backtest_run(e: &common::offline::Env, extra: &[&str]) -> String {
    let mut args = vec!["backtest", "run"];
    args.extend_from_slice(&RANGE);
    args.extend_from_slice(extra);
    let out = run(e, &args);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    stdout(&out)
}

fn value<'a>(text: &'a str, label: &str) -> &'a str {
    text.lines()
        .find_map(|l| {
            let mut it = l.split_whitespace();
            (it.next() == Some(label)).then(|| it.next()).flatten()
        })
        .unwrap_or_else(|| panic!("no {label} line in: {text}"))
}

#[test]
fn default_capital_is_the_configured_default_with_source_config() {
    // R-542
    let e = env();
    let text = backtest_run(&e, &[]);
    assert!(has_line(&text, "capital", "100000"), "stdout: {text}");
    assert!(has_line(&text, "source", "config"), "stdout: {text}");
}

#[test]
fn configured_initial_capital_is_used_with_source_config() {
    // R-542
    let e = env();
    let mut cfg = std::fs::read_to_string(&e.cfg).unwrap();
    cfg.push_str("\n[backtest]\ninitial_capital = 50000\n");
    std::fs::write(&e.cfg, cfg).unwrap();
    let text = backtest_run(&e, &[]);
    assert!(has_line(&text, "capital", "50000"), "stdout: {text}");
    assert!(has_line(&text, "source", "config"), "stdout: {text}");
}

#[test]
fn rerun_at_another_capital_is_the_same_trial() {
    // R-543
    let e = env();
    let first = backtest_run(&e, &[]);
    let second = backtest_run(&e, &["--capital", "250000"]);
    assert_eq!(value(&first, "trial"), value(&second, "trial"));
    assert_eq!(value(&second, "trials"), "1", "stdout: {second}");
}
