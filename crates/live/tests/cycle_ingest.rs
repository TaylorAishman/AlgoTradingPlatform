//! R-311, R-317: a run in the ingest stage only reads, and every run is logged.

mod common;

use common::{d, sym, LiveFixture};
use pt_config::hash::config_hash;
use pt_core::order_state::OrderState;
use pt_core::types::Adjustment;
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::{Book, Mode};

fn phases(outcome: &pt_live::cycle::RunOutcome) -> Vec<&str> {
    outcome.phases.iter().map(String::as_str).collect()
}

// R-311
#[test]
fn an_ingest_run_reads_calendar_bars_and_assets_and_nothing_else() {
    let fx = LiveFixture::new(Stage::Ingest);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    let ran = phases(&outcome);
    for expected in ["calendar", "ingest", "assets"] {
        assert!(ran.contains(&expected), "missing {expected}: {ran:?}");
    }
    for forbidden in ["rebalance", "submit"] {
        assert!(!ran.contains(&forbidden), "ran {forbidden}: {ran:?}");
    }
}

// R-311
#[test]
fn an_ingest_run_stores_bars_through_the_last_session() {
    let fx = LiveFixture::new(Stage::Ingest);

    run_cycle(&fx.ctx()).expect("run");

    for name in ["SPY", "TLT", "BIL"] {
        let bars = fx
            .store
            .bars_asof(
                &sym(name),
                Adjustment::Raw,
                "sip",
                d(2024, 1, 2),
                d(2024, 3, 1),
                None,
            )
            .expect("bars");
        assert_eq!(
            bars.last().map(|b| b.date),
            Some(d(2024, 2, 29)),
            "{name} bars end"
        );
    }
}

// R-311
#[test]
fn an_ingest_run_writes_no_decision_no_order_and_no_ledger_entry() {
    let fx = LiveFixture::new(Stage::Ingest);

    run_cycle(&fx.ctx()).expect("run");

    assert!(fx
        .store
        .decisions(Mode::DryRun)
        .expect("decisions")
        .is_empty());
    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
    assert!(fx
        .store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::DryRun,
            OrderState::Submitted,
            OrderState::PartiallyFilled,
            OrderState::Filled,
            OrderState::Canceled,
            OrderState::Expired,
            OrderState::Rejected,
            OrderState::Abandoned,
        ])
        .expect("orders")
        .is_empty());
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());
}

// R-311
#[test]
fn an_ingest_run_sends_only_get_requests() {
    let fx = LiveFixture::new(Stage::Ingest);

    run_cycle(&fx.ctx()).expect("run");

    let requests = fx.mock.requests();
    assert!(!requests.is_empty(), "the run read nothing");
    let writes: Vec<_> = requests.iter().filter(|r| r.method != "GET").collect();
    assert!(writes.is_empty(), "non-GET requests: {writes:?}");
}

// R-317
#[test]
fn a_run_is_logged_with_its_stage_status_hash_version_and_phases() {
    let fx = LiveFixture::new(Stage::Ingest);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    let runs = fx.store.runs().expect("runs");
    assert_eq!(runs.len(), 1, "{runs:?}");
    let row = &runs[0];
    assert_eq!(row.id, outcome.run_id);
    assert_eq!(row.stage, "ingest");
    assert_eq!(row.status, "ok");
    assert_eq!(row.config_hash, config_hash(&fx.cfg));
    assert_eq!(row.code_version, "test-version");
    assert_eq!(row.phases, outcome.phases.join(","));
    assert!(row.finished_at.is_some(), "run row was not finished");
}
