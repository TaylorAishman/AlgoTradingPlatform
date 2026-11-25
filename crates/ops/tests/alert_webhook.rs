//! Tests that an alert is posted to the webhook as JSON (R-351).

use chrono::{DateTime, TimeZone, Utc};
use pt_ops::alert::{Alerter, Level};
use pt_store::db::Store;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// R-351: the listener receives one JSON POST with the alert's fields, and the
// alert is stored as delivered.
#[test]
fn alert_is_posted_as_json_and_stored_delivered() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/hook", server.server_addr().to_ip().unwrap());
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let Some(mut req) = server.recv_timeout(Duration::from_secs(5)).unwrap() else {
            tx.send(("none".to_string(), None, String::new())).unwrap();
            return;
        };
        let mut body = String::new();
        std::io::Read::read_to_string(req.as_reader(), &mut body).unwrap();
        let content_type = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("Content-Type"))
            .map(|h| h.value.as_str().to_string());
        let method = req.method().to_string();
        req.respond(tiny_http::Response::empty(200)).unwrap();
        tx.send((method, content_type, body)).unwrap();
    });

    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(Some(url), vec![]);
    let sent = alerter.alert(&store, Level::Critical, "halt_set", "msg", Some(7), t);
    handle.join().unwrap();

    assert!(sent);
    let (method, content_type, body) = rx.recv().unwrap();
    assert_eq!(method, "POST");
    assert!(content_type.unwrap().starts_with("application/json"));
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["level"], "critical");
    assert_eq!(v["code"], "halt_set");
    assert_eq!(v["message"], "msg");
    assert_eq!(v["run_id"], 7);
    let ts = DateTime::parse_from_rfc3339(v["ts"].as_str().unwrap()).unwrap();
    assert_eq!(ts.with_timezone(&Utc), t);

    assert_eq!(
        store.alerts().unwrap(),
        vec![(
            "critical".to_string(),
            "halt_set".to_string(),
            "msg".to_string(),
            true
        )]
    );
}
