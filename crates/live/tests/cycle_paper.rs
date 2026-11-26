//! R-313, R-317: a paper-stage run ingests, reconciles, decides and submits in one pass.

mod common;

use common::LiveFixture;
use pt_config::hash::config_hash;
use pt_core::order_state::OrderState;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::{Book, Mode};

fn posted_symbols(fx: &LiveFixture) -> Vec<String> {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .map(|r| {
            assert_eq!(r.path, "/v2/orders");
            let body: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
            format!(
                "{} {}",
                body["side"].as_str().unwrap_or_default(),
                body["symbol"].as_str().unwrap_or_default()
            )
        })
        .collect()
}

// R-313
#[test]
fn a_paper_cycle_runs_every_phase_in_order_and_ends_ok() {
    let fx = LiveFixture::new(Stage::Paper);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    let ran: Vec<&str> = outcome.phases.iter().map(String::as_str).collect();
    assert_eq!(
        ran,
        vec![
            "calendar",
            "ingest",
            "assets",
            "settle",
            "reconcile",
            "snapshot",
            "halts",
            "rebalance",
            "submit"
        ]
    );
}

// R-313
#[test]
fn a_paper_cycle_opens_the_ledger_and_records_one_planned_decision() {
    let fx = LiveFixture::new(Stage::Paper);

    run_cycle(&fx.ctx()).expect("run");

    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1, "{ledger:?}");
    assert_eq!(ledger[0].cash_delta.to_string(), "100000");
    let decisions = fx.store.decisions(Mode::Paper).expect("decisions");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].status, "planned");
    assert!(fx
        .store
        .decisions(Mode::DryRun)
        .expect("decisions")
        .is_empty());
}

// R-313
#[test]
fn a_paper_cycle_submits_both_orders() {
    let fx = LiveFixture::new(Stage::Paper);

    run_cycle(&fx.ctx()).expect("run");

    let submitted = fx
        .store
        .orders_in_states(&[OrderState::Submitted])
        .expect("orders");
    assert_eq!(submitted.len(), 2, "{submitted:?}");
    let other = fx
        .store
        .orders_in_states(&[OrderState::Intent, OrderState::DryRun, OrderState::Rejected])
        .expect("orders");
    assert!(other.is_empty(), "{other:?}");
}

// R-313
#[test]
fn a_paper_cycle_posts_exactly_two_orders_with_the_tlt_sell_first() {
    let fx = LiveFixture::new(Stage::Paper);

    run_cycle(&fx.ctx()).expect("run");

    assert_eq!(posted_symbols(&fx), vec!["sell TLT", "buy SPY"]);
}

// R-317
#[test]
fn a_paper_run_is_logged_with_stage_paper_and_what_it_ran() {
    let fx = LiveFixture::new(Stage::Paper);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    let runs = fx.store.runs().expect("runs");
    assert_eq!(runs.len(), 1, "{runs:?}");
    let row = &runs[0];
    assert_eq!(row.id, outcome.run_id);
    assert_eq!(row.stage, "paper");
    assert_eq!(row.status, "ok");
    assert_eq!(row.config_hash, config_hash(&fx.cfg));
    assert_eq!(row.code_version, "test-version");
    assert_eq!(row.phases, outcome.phases.join(","));
    assert!(row.finished_at.is_some(), "run row was not finished");
}
