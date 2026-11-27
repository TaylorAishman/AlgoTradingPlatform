//! R-544: with keys set and stage `ingest` or higher, `backtest run` uses the account's equity.
//! R-545: in the backtest stage it makes no request.
//! R-546: a failed account read falls back to the configured capital with a warning.

mod common;

use common::offline::{env, Env};
use common::{stderr, stdout};
use pt_mock_alpaca::server::MockAlpaca;
use std::process::Output;

fn has_line(text: &str, label: &str, value: &str) -> bool {
    text.lines()
        .any(|l| l.split_whitespace().collect::<Vec<_>>() == [label, value])
}

fn set_stage(e: &Env, stage: &str) {
    std::fs::write(
        e.home.path().join("owner.toml"),
        format!("stage = \"{stage}\"\n"),
    )
    .unwrap();
}

/// `backtest run` over the fixture range with fake keys pointing at the mock;
/// the owner file is passed only when the test wrote one.
fn run_with_keys(e: &Env, mock: &MockAlpaca) -> Output {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.arg("--config")
        .arg(&e.cfg)
        .args([
            "backtest",
            "run",
            "--from",
            "2024-03-01",
            "--to",
            "2024-12-30",
        ])
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("APCA_API_KEY_ID", "fake-key-id")
        .env("APCA_API_SECRET_KEY", "fake-secret")
        .env("PT_ALPACA_TRADING_URL", mock.url())
        .env("PT_ALPACA_DATA_URL", mock.url())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE");
    let owner = e.home.path().join("owner.toml");
    if owner.exists() {
        cmd.env("PT_OWNER_FILE", owner);
    } else {
        cmd.env_remove("PT_OWNER_FILE");
    }
    cmd.output().unwrap()
}

#[test]
fn account_equity_is_the_capital_at_ingest_stage_with_one_account_read() {
    // R-544
    let e = env();
    let mock = MockAlpaca::start();
    mock.seed_account("123456.78");
    set_stage(&e, "ingest");
    let out = run_with_keys(&e, &mock);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(has_line(&text, "capital", "123456.78"), "stdout: {text}");
    assert!(has_line(&text, "source", "account"), "stdout: {text}");
    let reqs: Vec<(String, String)> = mock
        .requests()
        .into_iter()
        .map(|r| (r.method, r.path))
        .collect();
    assert_eq!(reqs, [("GET".to_string(), "/v2/account".to_string())]);
}

#[test]
fn backtest_stage_makes_no_request_and_logs_why() {
    // R-545
    let e = env();
    let mock = MockAlpaca::start();
    mock.seed_account("123456.78");
    let out = run_with_keys(&e, &mock);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(has_line(&text, "source", "config"), "stdout: {text}");
    assert!(has_line(&text, "capital", "100000"), "stdout: {text}");
    assert!(
        mock.requests().is_empty(),
        "requests: {:?}",
        mock.requests()
    );
    let err = stderr(&out);
    let logged = err.lines().any(|l| {
        serde_json::from_str::<serde_json::Value>(l)
            .ok()
            .and_then(|v| v["message"].as_str().map(|m| m.contains("backtest")))
            .unwrap_or(false)
    });
    assert!(logged, "no log line mentioning backtest in: {err}");
}

#[test]
fn failed_account_read_falls_back_to_config_with_a_warning() {
    // R-546
    let e = env();
    let mock = MockAlpaca::start();
    mock.seed_account("123456.78");
    mock.fail_next(100, 500);
    set_stage(&e, "ingest");
    let out = run_with_keys(&e, &mock);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(has_line(&text, "capital", "100000"), "stdout: {text}");
    assert!(has_line(&text, "source", "config"), "stdout: {text}");
    let err = stderr(&out);
    let warned = err.lines().any(|l| {
        serde_json::from_str::<serde_json::Value>(l)
            .ok()
            .is_some_and(|v| v["level"] == "warn")
    });
    assert!(warned, "no warn line in: {err}");
}
