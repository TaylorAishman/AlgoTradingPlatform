//! R-541: `backtest run` takes its starting capital from `--capital`.

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
