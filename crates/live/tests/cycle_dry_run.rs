//! R-323: a dry-run stage run records a decision and sends nothing.

mod common;

use common::LiveFixture;
use pt_core::order_state::OrderState;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::{Book, Mode};

fn dry_run_orders(fx: &LiveFixture) -> usize {
    fx.store
        .orders_in_states(&[OrderState::DryRun])
        .expect("orders")
        .len()
}

// R-323
#[test]
fn a_dry_run_cycle_runs_the_read_phases_then_rebalance() {
    let fx = LiveFixture::new(Stage::DryRun);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    let ran: Vec<&str> = outcome.phases.iter().map(String::as_str).collect();
    assert_eq!(ran, vec!["calendar", "ingest", "assets", "rebalance"]);
}

// R-323
#[test]
fn a_dry_run_cycle_records_one_dry_run_decision_and_two_dry_run_orders() {
    let fx = LiveFixture::new(Stage::DryRun);

    run_cycle(&fx.ctx()).expect("run");

    let decisions = fx.store.decisions(Mode::DryRun).expect("decisions");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].mode, Mode::DryRun);
    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
    assert_eq!(dry_run_orders(&fx), 2);
}

// R-323
#[test]
fn a_dry_run_cycle_writes_no_ledger_entry_and_sends_only_get_requests() {
    let fx = LiveFixture::new(Stage::DryRun);

    run_cycle(&fx.ctx()).expect("run");

    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());
    let requests = fx.mock.requests();
    assert!(!requests.is_empty(), "the run read nothing");
    let writes: Vec<_> = requests.iter().filter(|r| r.method != "GET").collect();
    assert!(writes.is_empty(), "non-GET requests: {writes:?}");
}

// R-323
#[test]
fn a_second_dry_run_cycle_adds_nothing() {
    let fx = LiveFixture::new(Stage::DryRun);
    run_cycle(&fx.ctx()).expect("first run");

    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(
        fx.store.decisions(Mode::DryRun).expect("decisions").len(),
        1
    );
    assert_eq!(dry_run_orders(&fx), 2);
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());
    assert!(fx.mock.requests().iter().all(|r| r.method == "GET"));
}
