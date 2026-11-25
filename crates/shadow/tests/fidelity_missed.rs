//! R-339: a session whose snapshot was first recorded late counts as missed.

mod common;

use chrono::{DateTime, NaiveDate, Utc};
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_shadow::fidelity::fidelity;
use pt_store::db::Store;
use pt_store::trading::{Book, EquitySnapshot};
use pt_strategy::benchmark::LongOnly;
use std::sync::Arc;

fn store_with(snapshots: &[(NaiveDate, DateTime<Utc>)]) -> Store {
    let store = Store::open_in_memory().unwrap();
    store
        .put_sessions(&[d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)])
        .unwrap();
    for (date, recorded_at) in snapshots {
        store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date: *date,
                cash: dec("100000"),
                positions_value: dec("0"),
                equity: dec("100000"),
                recorded_at: *recorded_at,
            })
            .unwrap();
    }
    store
}

fn missed(snapshots: &[(NaiveDate, DateTime<Utc>)]) -> usize {
    let store = store_with(snapshots);
    let sessions = vec![d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)];
    let strategy = LongOnly::new(Arc::new(TradingCalendar::new(sessions).unwrap()));
    fidelity(&store, &config(), &strategy)
        .unwrap()
        .missed_sessions
}

// R-339
#[test]
fn a_snapshot_recorded_after_the_next_session_counts_as_missed() {
    let n = missed(&[
        (d(2024, 1, 3), at(2024, 1, 4, 13, 0)),
        (d(2024, 1, 4), at(2024, 1, 8, 13, 0)),
    ]);

    assert_eq!(n, 1);
}

// R-339
#[test]
fn a_snapshot_recorded_on_the_next_sessions_date_is_on_time() {
    let n = missed(&[
        (d(2024, 1, 2), at(2024, 1, 2, 21, 0)),
        (d(2024, 1, 3), at(2024, 1, 4, 23, 59)),
        (d(2024, 1, 4), at(2024, 1, 5, 0, 0)),
    ]);

    assert_eq!(n, 0);
}

// R-339
#[test]
fn every_late_snapshot_is_counted() {
    let n = missed(&[
        (d(2024, 1, 2), at(2024, 1, 4, 9, 0)),
        (d(2024, 1, 3), at(2024, 1, 5, 9, 0)),
        (d(2024, 1, 4), at(2024, 1, 4, 21, 0)),
    ]);

    assert_eq!(n, 2);
}

// R-339
#[test]
fn the_shadow_fixture_has_no_missed_sessions() {
    let store = shadow_store();
    let sessions = vec![d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)];
    let strategy = LongOnly::new(Arc::new(TradingCalendar::new(sessions).unwrap()));

    let f = fidelity(&store, &config(), &strategy).unwrap();

    assert_eq!(f.missed_sessions, 0);
}
