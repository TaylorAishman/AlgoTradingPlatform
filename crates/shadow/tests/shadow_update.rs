//! R-335: a day without the snapshots it needs is not compared, and `update`
//! stores every session not yet compared exactly once.

mod common;

use common::*;
use pt_shadow::daily::{shadow_day, update};
use pt_shadow::error::ShadowError;

// R-335
#[test]
fn day_without_a_snapshot_is_not_found() {
    let store = shadow_store();
    let cfg = config();

    let result = shadow_day(&store, &cfg, d(2024, 1, 5));

    assert!(
        matches!(result, Err(ShadowError::NotFound(_))),
        "{result:?}"
    );
}

// R-335
#[test]
fn update_stores_the_two_comparable_sessions_and_then_nothing() {
    let store = shadow_store();
    let cfg = config();

    assert_eq!(update(&store, &cfg).unwrap(), 2);

    let dates: Vec<_> = store
        .shadow_days()
        .unwrap()
        .into_iter()
        .map(|row| row.date)
        .collect();
    assert_eq!(dates, vec![d(2024, 1, 3), d(2024, 1, 4)]);

    assert_eq!(update(&store, &cfg).unwrap(), 0);
    assert_eq!(store.shadow_days().unwrap().len(), 2);
}
