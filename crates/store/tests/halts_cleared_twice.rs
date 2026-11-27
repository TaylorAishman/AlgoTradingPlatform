//! R-428: clearing a halt that is already cleared is an error that says so.

use chrono::{DateTime, TimeZone, Utc};
use pt_store::db::Store;
use pt_store::error::StoreError;

fn t(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, h, m, 0).unwrap()
}

#[test]
fn second_clear_says_already_cleared_and_keeps_first_clearing() {
    // R-428
    let store = Store::open_in_memory().unwrap();
    let id = store.set_halt("daily_loss", "detail", t(20, 0)).unwrap();
    store.clear_halt(id, "checked", t(21, 0)).unwrap();
    let err = store.clear_halt(id, "again", t(22, 0)).unwrap_err();
    match err {
        StoreError::NotFound(msg) => {
            assert!(msg.contains("already cleared"), "message was: {msg}")
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
    let all = store.halts().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].cleared_at, Some(t(21, 0)));
    assert_eq!(all[0].clear_note, Some("checked".to_string()));
}

#[test]
fn clearing_an_unknown_halt_is_not_found_without_already_cleared() {
    // R-428
    let store = Store::open_in_memory().unwrap();
    let err = store.clear_halt(999, "x", t(21, 0)).unwrap_err();
    match err {
        StoreError::NotFound(msg) => {
            assert!(!msg.contains("already cleared"), "message was: {msg}")
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}
