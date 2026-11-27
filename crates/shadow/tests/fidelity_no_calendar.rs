//! R-518: the fidelity of a store with no calendar is all zero, not an error.

mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_shadow::fidelity::fidelity;
use pt_store::db::Store;
use pt_strategy::benchmark::LongOnly;
use std::sync::Arc;

// R-518
#[test]
fn a_new_store_with_no_calendar_has_all_zero_fidelity() {
    let store = Store::open_in_memory().unwrap();
    let cfg = config();
    let sessions = vec![d(2024, 1, 2), d(2024, 1, 3)];
    let strategy = LongOnly::new(Arc::new(TradingCalendar::new(sessions).unwrap()));

    let f = fidelity(&store, &cfg, &strategy).expect("no calendar is not an error");

    assert_eq!(f.n_sessions, 0);
    assert_eq!(f.n_rebalances, 0);
    assert_eq!(f.n_fills, 0);
    assert_eq!(f.replay_mismatches, 0);
    assert_eq!(f.reconciliation_breaks, 0);
    assert_eq!(f.missed_sessions, 0);
    assert_eq!(f.late_rebalances, 0);
    assert_eq!(f.mean_slippage_bps, 0.0);
    assert_eq!(f.mean_diff_bps, 0.0);
    assert_eq!(f.max_abs_diff_bps, 0.0);
}
