//! R-390: the cross-check is runnable from the command line and fails loudly.

mod common;

use common::offline::{env_with_max_order_notional, run, Env};
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

const RANGE: [&str; 4] = ["--from", "2024-03-01", "--to", "2024-12-30"];

fn crosscheck(e: &Env, extra: &[&str]) -> Output {
    let mut args = vec!["backtest", "crosscheck"];
    args.extend(RANGE);
    args.extend(extra);
    run(e, &args)
}

#[test]
fn crosscheck_passes_with_zero_max_abs_diff() {
    // R-390
    let e = env_with_max_order_notional(1_000_000);
    let out = crosscheck(&e, &[]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("PASS"), "stdout: {}", stdout(&out));
    let toks = tokens(&out);
    assert!(
        toks.windows(2)
            .any(|w| w[0] == "max_abs_diff" && w[1] == "0"),
        "no `max_abs_diff 0` in: {}",
        stdout(&out)
    );
}

#[test]
fn crosscheck_with_costs_passes() {
    // R-390
    let e = env_with_max_order_notional(1_000_000);
    let out = crosscheck(&e, &["--with-costs"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("PASS"), "stdout: {}", stdout(&out));
}

#[test]
fn crosscheck_exits_one_and_says_not_comparable_when_orders_are_capped() {
    // R-390
    let e = env_with_max_order_notional(100);
    let out = crosscheck(&e, &[]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    let all = format!("{}{}", stdout(&out), stderr(&out)).to_lowercase();
    assert!(all.contains("not comparable"), "output: {all}");
}
