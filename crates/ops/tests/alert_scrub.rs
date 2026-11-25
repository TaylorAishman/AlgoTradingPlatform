//! Tests that secrets are scrubbed from alerts (R-353).

use chrono::{TimeZone, Utc};
use pt_ops::alert::{Alerter, Level};
use pt_store::db::Store;
use std::thread;
use std::time::Duration;

// R-353: a secret in the message is replaced by *** in the stored alert and
// in the body posted to the webhook.
#[test]
fn secret_is_scrubbed_from_stored_and_posted_alert() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hook", server.server_addr().to_ip().unwrap());
    let handle = thread::spawn(move || {
        let mut req = server
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .expect("webhook should receive a POST");
        let mut body = String::new();
        req.as_reader().read_to_string(&mut body).unwrap();
        req.respond(tiny_http::Response::empty(200)).unwrap();
        body
    });

    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(Some(url), vec!["FAKEKEY123".to_string()]);

    let sent = alerter.alert(
        &store,
        Level::Warn,
        "feed_down",
        "failed with key FAKEKEY123",
        None,
        t,
    );
    let body = handle.join().unwrap();

    assert!(sent);
    assert!(!body.contains("FAKEKEY123"), "posted body leaked: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["message"], "failed with key ***");
    assert_eq!(
        store.alerts().unwrap(),
        vec![(
            "warn".to_string(),
            "feed_down".to_string(),
            "failed with key ***".to_string(),
            true
        )]
    );
}

// R-353: every occurrence is replaced, and with no webhook the stored alert
// is still scrubbed.
#[test]
fn every_occurrence_is_scrubbed_without_webhook() {
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(None, vec!["FAKEKEY123".to_string()]);

    let sent = alerter.alert(
        &store,
        Level::Info,
        "c",
        "FAKEKEY123 and FAKEKEY123 again",
        None,
        t,
    );

    assert!(!sent);
    let stored = store.alerts().unwrap();
    assert_eq!(stored[0].2, "*** and *** again");
}
