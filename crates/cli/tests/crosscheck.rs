//! R-390: the cross-check is runnable from the command line and fails loudly.
//! R-547: it takes `--capital` and never reads the account.

mod common;

use common::offline::{env_with_max_order_notional, run, Env};
use common::{stderr, stdout};
use pt_mock_alpaca::server::MockAlpaca;
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

fn has_line(text: &str, label: &str, value: &str) -> bool {
    text.lines()
        .any(|l| l.split_whitespace().collect::<Vec<_>>() == [label, value])
}

#[test]
fn crosscheck_capital_flag_is_used_and_printed() {
    // R-547
    let e = env_with_max_order_notional(1_000_000);
    let out = crosscheck(&e, &["--capital", "250000"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("PASS"), "stdout: {text}");
    assert!(has_line(&text, "capital", "250000"), "stdout: {text}");
}

#[test]
fn crosscheck_with_keys_at_ingest_uses_config_capital_and_makes_no_request() {
    // R-547
    let e = env_with_max_order_notional(1_000_000);
    let mock = MockAlpaca::start();
    mock.seed_account("123456.78");
    let owner = e.home.path().join("owner.toml");
    std::fs::write(&owner, "stage = \"ingest\"\n").unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args(["backtest", "crosscheck"])
        .args(RANGE)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("APCA_API_KEY_ID", "fake-key-id")
        .env("APCA_API_SECRET_KEY", "fake-secret")
        .env("PT_ALPACA_TRADING_URL", mock.url())
        .env("PT_ALPACA_DATA_URL", mock.url())
        .env("PT_OWNER_FILE", owner)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(has_line(&text, "capital", "100000"), "stdout: {text}");
    assert!(
        mock.requests().is_empty(),
        "requests: {:?}",
        mock.requests()
    );
}
