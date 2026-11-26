//! R-320: the first order is never repeated, whether it filled, a day passed, or it was for another day.

mod common;

use chrono::{NaiveDate, TimeZone, Utc};
use common::{d, sym, LiveFixture};
use pt_core::clock::FixedClock;
use pt_core::order_state::OrderState;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::Book;

const FIRST_ORDER_ID: &str = "pt-first-20240301-SPY";

fn approved_fixture(valid_on: NaiveDate) -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::FirstOrder);
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty: 1,
            max_qty: 1,
            valid_on,
        }),
        ..OwnerFile::default()
    });
    fx
}

fn post_count(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .count()
}

/// Bars for the session 2024-03-01, as the feed has them once it has closed.
fn seed_march_first_bars(fx: &LiveFixture) {
    for adjustment in ["raw", "all"] {
        let rows = ["SPY", "TLT", "BIL"]
            .iter()
            .map(|name| {
                (
                    name.to_string(),
                    "2024-03-01".to_string(),
                    100.0,
                    100.0,
                    100.0,
                    100.0,
                    1_000_000u64,
                )
            })
            .collect();
        fx.mock.seed_bars(adjustment, "sip", rows);
    }
}

/// After the close of 2024-03-01: the broker's clock and the fixture's clock.
fn after_the_close(fx: &mut LiveFixture) {
    fx.mock.set_clock(
        "2024-03-01T17:00:00-05:00",
        false,
        "2024-03-04T09:30:00-05:00",
        "2024-03-04T16:00:00-05:00",
    );
    fx.clock = FixedClock(Utc.with_ymd_and_hms(2024, 3, 1, 22, 0, 0).unwrap());
}

/// Before the open of the next session, 2024-03-04.
fn next_morning(fx: &mut LiveFixture) {
    fx.mock.set_clock(
        "2024-03-04T08:00:00-05:00",
        false,
        "2024-03-04T09:30:00-05:00",
        "2024-03-04T16:00:00-05:00",
    );
    fx.clock = FixedClock(Utc.with_ymd_and_hms(2024, 3, 4, 13, 0, 0).unwrap());
}

// R-320
#[test]
fn a_run_after_the_fill_settles_it_and_sends_nothing_more() {
    let mut fx = approved_fixture(d(2024, 3, 1));
    run_cycle(&fx.ctx()).expect("first run");
    assert_eq!(post_count(&fx), 1);
    fx.mock.fill(FIRST_ORDER_ID, 1, "100");
    seed_march_first_bars(&fx);
    after_the_close(&mut fx);

    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(post_count(&fx), 1, "second run posted");
    let filled = fx
        .store
        .orders_in_states(&[OrderState::Filled])
        .expect("orders");
    assert_eq!(filled.len(), 1, "{filled:?}");
    assert_eq!(filled[0].client_order_id, FIRST_ORDER_ID);
    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.get(&sym("SPY")), Some(&1), "{positions:?}");
}

// R-320
#[test]
fn the_run_after_the_fill_reconciles_and_writes_the_snapshot() {
    let mut fx = approved_fixture(d(2024, 3, 1));
    run_cycle(&fx.ctx()).expect("first run");
    fx.mock.fill(FIRST_ORDER_ID, 1, "100");
    seed_march_first_bars(&fx);
    after_the_close(&mut fx);

    let second = run_cycle(&fx.ctx()).expect("second run");

    let ran: Vec<&str> = second.phases.iter().map(String::as_str).collect();
    assert!(ran.contains(&"reconcile"), "{ran:?}");
    assert!(ran.contains(&"snapshot"), "{ran:?}");
    assert!(fx.store.active_halts().expect("halts").is_empty());
    let series = fx.store.equity_series(Book::Paper).expect("series");
    assert_eq!(
        series.last().map(|s| s.date),
        Some(d(2024, 3, 1)),
        "{series:?}"
    );
}

// R-320
#[test]
fn a_third_run_on_the_next_day_sends_nothing() {
    let mut fx = approved_fixture(d(2024, 3, 1));
    run_cycle(&fx.ctx()).expect("first run");
    fx.mock.fill(FIRST_ORDER_ID, 1, "100");
    seed_march_first_bars(&fx);
    after_the_close(&mut fx);
    run_cycle(&fx.ctx()).expect("second run");
    next_morning(&mut fx);

    let third = run_cycle(&fx.ctx()).expect("third run");

    assert_eq!(third.status, RunStatus::Ok);
    assert_eq!(post_count(&fx), 1, "third run posted");
}

// R-320
#[test]
fn an_approval_for_another_day_sends_nothing_on_the_first_run() {
    let fx = approved_fixture(d(2024, 3, 4));

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    assert_eq!(post_count(&fx), 0);
    let orders = fx
        .store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::DryRun,
            OrderState::Rejected,
        ])
        .expect("orders");
    assert!(orders.is_empty(), "{orders:?}");
}
