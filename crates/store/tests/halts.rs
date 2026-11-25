//! R-069: halts persist until cleared and keep their history.

use chrono::{DateTime, TimeZone, Utc};
use pt_store::db::Store;
use pt_store::error::StoreError;

fn t(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, h, m, 0).unwrap()
}

#[test]
fn set_halt_is_active_until_cleared() {
    // R-069
    let store = Store::open_in_memory().unwrap();
    let id = store.set_halt("daily_loss", "detail", t(20, 0)).unwrap();
    let active = store.active_halts().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, id);
    assert_eq!(active[0].reason, "daily_loss");
    assert_eq!(active[0].detail, "detail");
    assert_eq!(active[0].set_at, t(20, 0));
    assert_eq!(active[0].cleared_at, None);
    assert_eq!(active[0].clear_note, None);
}

#[test]
fn cleared_halt_leaves_active_but_stays_in_history() {
    // R-069
    let store = Store::open_in_memory().unwrap();
    let id = store.set_halt("daily_loss", "detail", t(20, 0)).unwrap();
    store.clear_halt(id, "checked", t(21, 0)).unwrap();
    assert!(store.active_halts().unwrap().is_empty());
    let all = store.halts().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, id);
    assert_eq!(all[0].cleared_at, Some(t(21, 0)));
    assert_eq!(all[0].clear_note, Some("checked".to_string()));
}

#[test]
fn clearing_one_halt_keeps_the_other_active() {
    // R-069
    let store = Store::open_in_memory().unwrap();
    let a = store.set_halt("daily_loss", "a", t(20, 0)).unwrap();
    let b = store.set_halt("drawdown", "b", t(20, 1)).unwrap();
    assert_ne!(a, b);
    store.clear_halt(a, "ok", t(21, 0)).unwrap();
    let active = store.active_halts().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, b);
    assert_eq!(store.halts().unwrap().len(), 2);
}

#[test]
fn clearing_an_unknown_halt_is_not_found() {
    // R-069
    let store = Store::open_in_memory().unwrap();
    let err = store.clear_halt(999, "x", t(21, 0)).unwrap_err();
    assert!(matches!(err, StoreError::NotFound(_)), "got {err:?}");
}
