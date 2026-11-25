//! Tests that an alert is always stored, with or without a webhook (R-350).

use chrono::{TimeZone, Utc};
use pt_ops::alert::{Alerter, Level};
use pt_store::db::Store;

// R-350: with no webhook the alert returns false and is stored undelivered.
#[test]
fn alert_without_webhook_is_stored_undelivered() {
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(None, vec![]);
    let sent = alerter.alert(&store, Level::Warn, "data_not_ready", "msg", Some(7), t);
    assert!(!sent);
    assert_eq!(
        store.alerts().unwrap(),
        vec![(
            "warn".to_string(),
            "data_not_ready".to_string(),
            "msg".to_string(),
            false
        )]
    );
}

// R-350: every call adds a row, whatever the level.
#[test]
fn each_alert_adds_one_row_with_its_level() {
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(None, vec![]);
    alerter.alert(&store, Level::Info, "a", "one", None, t);
    alerter.alert(&store, Level::Critical, "b", "two", None, t);
    let rows = store.alerts().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "info");
    assert_eq!(rows[1].0, "critical");
}
