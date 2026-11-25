//! R-070: alerts are recorded.

use chrono::{DateTime, TimeZone, Utc};
use pt_store::db::Store;

fn t(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, h, 0, 0).unwrap()
}

fn tuple(level: &str, code: &str, msg: &str, delivered: bool) -> (String, String, String, bool) {
    (
        level.to_string(),
        code.to_string(),
        msg.to_string(),
        delivered,
    )
}

#[test]
fn recorded_alert_is_listed() {
    // R-070
    let store = Store::open_in_memory().unwrap();
    assert!(store.alerts().unwrap().is_empty());
    store
        .record_alert("critical", "halt_set", "msg", false, t(20))
        .unwrap();
    assert_eq!(
        store.alerts().unwrap(),
        vec![tuple("critical", "halt_set", "msg", false)]
    );
}

#[test]
fn alerts_keep_order_and_delivery_flag() {
    // R-070
    let store = Store::open_in_memory().unwrap();
    let a = store
        .record_alert("critical", "halt_set", "one", false, t(20))
        .unwrap();
    let b = store
        .record_alert("warning", "missed_run", "two", true, t(21))
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        store.alerts().unwrap(),
        vec![
            tuple("critical", "halt_set", "one", false),
            tuple("warning", "missed_run", "two", true)
        ]
    );
}
