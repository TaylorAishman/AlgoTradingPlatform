//! R-538: the status report shows the fidelity summary after the mock paper session.

mod common;

use common::live::{env, posts, run, Env};
use common::{stderr, stdout};
use std::process::Output;

const LABELS: [&str; 10] = [
    "n_sessions",
    "n_rebalances",
    "n_fills",
    "replay_mismatches",
    "reconciliation_breaks",
    "missed_sessions",
    "late_rebalances",
    "mean_slippage_bps",
    "mean_diff_bps",
    "max_abs_diff_bps",
];

fn ok(e: &Env, args: &[&str]) -> Output {
    let out = run(e, args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?}\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
    out
}

/// Plays the paper session: a run before the open, both fills, bars, a run after the close.
fn paper_session(e: &Env) {
    // (1) pre-open run: exits 0 and submits two orders.
    ok(e, &["run"]);
    let sent = posts(e);
    assert_eq!(sent.len(), 2, "POSTs: {sent:?}");

    // (2) fill both orders at the mock, seed 2024-03-01 bars, set a post-close clock.
    for p in &sent {
        let body: serde_json::Value = serde_json::from_str(&p.body).unwrap();
        let text = |k: &str| body[k].as_str().unwrap_or_default().to_string();
        let qty: u64 = text("qty").parse().expect("integer qty");
        assert!(qty > 0, "{body}");
        let symbol = text("symbol");
        let price = if symbol == "SPY" { "140.00" } else { "80.20" };
        e.mock.fill(&text("client_order_id"), qty, price);
    }
    e.mock.seed_bars(
        "raw",
        "sip",
        vec![
            (
                "SPY".into(),
                "2024-03-01".into(),
                141.0,
                142.0,
                141.0,
                142.0,
                1_000_000u64,
            ),
            (
                "TLT".into(),
                "2024-03-01".into(),
                80.0,
                80.0,
                79.9,
                79.9,
                1_000_000,
            ),
            (
                "BIL".into(),
                "2024-03-01".into(),
                100.4,
                100.41,
                100.4,
                100.41,
                1_000_000,
            ),
        ],
    );
    e.mock.seed_bars(
        "all",
        "sip",
        vec![
            (
                "SPY".into(),
                "2024-03-01".into(),
                141.0,
                142.0,
                141.0,
                142.0,
                1_000_000u64,
            ),
            (
                "TLT".into(),
                "2024-03-01".into(),
                80.0,
                80.0,
                79.9,
                79.9,
                1_000_000,
            ),
            (
                "BIL".into(),
                "2024-03-01".into(),
                100.4,
                100.41,
                100.4,
                100.41,
                1_000_000,
            ),
        ],
    );
    e.mock.set_clock(
        "2024-03-01T17:30:00-05:00",
        false,
        "2024-03-04T09:30:00-05:00",
        "2024-03-04T16:00:00-05:00",
    );

    // (3) post-close run.
    ok(e, &["run"]);
}

fn fidelity_of(e: &Env) -> Vec<(String, String)> {
    let path = e.dir.path().join("reports").join("status.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("status.json exists"))
            .expect("status.json is JSON");
    json["fidelity"]
        .as_array()
        .expect("fidelity array")
        .iter()
        .map(|f| {
            let value = match &f["value"] {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            (f["label"].as_str().expect("label").to_string(), value)
        })
        .collect()
}

fn check(f: &[(String, String)]) {
    let labels: Vec<&str> = f.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(labels, LABELS, "ten labels in order: {f:?}");
    let value = |l: &str| f.iter().find(|(k, _)| k == l).unwrap().1.clone();
    assert_eq!(value("n_fills"), "2", "{f:?}");
    assert_eq!(value("replay_mismatches"), "0", "{f:?}");
}

#[test]
fn status_json_after_the_paper_session_has_the_ten_fidelity_elements() {
    // R-538
    let e = env();
    paper_session(&e);
    check(&fidelity_of(&e));
}

#[test]
fn trader_report_afterwards_writes_the_same_fidelity_array() {
    // R-538
    let e = env();
    paper_session(&e);
    let from_run = fidelity_of(&e);
    ok(&e, &["report"]);
    let from_report = fidelity_of(&e);
    check(&from_report);
    assert_eq!(from_report, from_run);
}
