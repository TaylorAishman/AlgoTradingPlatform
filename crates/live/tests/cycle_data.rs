//! R-318: a run that finds the data not ready still settles and reconciles, then stops.

mod common;

use common::{sessions_2024, LiveFixture};
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::Mode;

/// A paper fixture whose feed has every bar through 2024-02-29 except TLT's on that day.
fn missing_tlt_fixture() -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::Paper);
    let mut rows = Vec::new();
    for (i, date) in sessions_2024().iter().take(41).enumerate() {
        for (name, base) in [("SPY", 100.0), ("TLT", 90.0), ("BIL", 100.0)] {
            if i == 40 && name == "TLT" {
                continue;
            }
            let close = base + i as f64;
            rows.push((
                name.to_string(),
                date.to_string(),
                close,
                close,
                close,
                close,
                1_000_000u64,
            ));
        }
    }
    fx.mock.seed_bars("raw", "gap", rows.clone());
    fx.mock.seed_bars("all", "gap", rows);
    fx.cfg.data.feed = "gap".to_string();
    fx
}

// R-318
#[test]
fn a_run_with_a_missing_tlt_bar_ends_data_not_ready() {
    let fx = missing_tlt_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::DataNotReady);
}

// R-318
#[test]
fn a_data_not_ready_run_still_settles_and_reconciles_but_never_submits() {
    let fx = missing_tlt_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    let ran: Vec<&str> = outcome.phases.iter().map(String::as_str).collect();
    assert!(ran.contains(&"settle"), "{ran:?}");
    assert!(ran.contains(&"reconcile"), "{ran:?}");
    assert!(!ran.contains(&"submit"), "{ran:?}");
}

// R-318
#[test]
fn a_data_not_ready_run_is_logged_with_that_status() {
    let fx = missing_tlt_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let runs = fx.store.runs().expect("runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "data_not_ready");
    assert!(runs[0].finished_at.is_some(), "run row was not finished");
}

// R-318
#[test]
fn a_data_not_ready_run_writes_no_decision() {
    let fx = missing_tlt_fixture();

    run_cycle(&fx.ctx()).expect("run");

    for mode in [Mode::Paper, Mode::DryRun] {
        assert!(fx.store.decisions(mode).expect("decisions").is_empty());
    }
}

// R-318
#[test]
fn a_data_not_ready_run_sends_no_post_to_the_broker() {
    let fx = missing_tlt_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let posts: Vec<_> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert!(posts.is_empty(), "{posts:?}");
}
