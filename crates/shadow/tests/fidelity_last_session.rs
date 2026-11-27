//! R-519: the last session never counts as missed.

mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_shadow::fidelity::fidelity;
use pt_store::db::Store;
use pt_store::trading::{Book, EquitySnapshot};
use pt_strategy::benchmark::LongOnly;
use std::sync::Arc;

// R-519
#[test]
fn a_very_late_snapshot_for_the_last_session_is_not_missed() {
    let sessions = vec![d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4)];
    let store = Store::open_in_memory().unwrap();
    store.put_sessions(&sessions).unwrap();
    for (date, recorded_at) in [
        (d(2024, 1, 2), at(2024, 1, 5, 13, 0)),
        (d(2024, 1, 3), at(2024, 1, 4, 13, 0)),
        (d(2024, 1, 4), at(2024, 1, 20, 13, 0)),
    ] {
        store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date,
                cash: dec("100000"),
                positions_value: dec("0"),
                equity: dec("100000"),
                recorded_at,
            })
            .unwrap();
    }
    let strategy = LongOnly::new(Arc::new(TradingCalendar::new(sessions).unwrap()));

    let f = fidelity(&store, &config(), &strategy).unwrap();

    assert_eq!(f.missed_sessions, 1);
}
