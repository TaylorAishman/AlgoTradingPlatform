//! Tests that secrets are scrubbed from an alert's code (R-522).

use chrono::{TimeZone, Utc};
use pt_ops::alert::{Alerter, Level};
use pt_store::db::Store;
use std::process::Command;
use std::thread;
use std::time::Duration;

const SECRET: &str = "s3cr3t-value";
const CODE: &str = "bad_s3cr3t-value";
const CHILD_ENV: &str = "PT_ALERT_SCRUB_CODE_CHILD";

// R-522: a secret in the code is scrubbed from the stored code and from the
// body posted to the webhook.
#[test]
fn secret_in_code_is_scrubbed_from_stored_and_posted_alert() {
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
    let alerter = Alerter::new(Some(url), vec![SECRET.to_string()]);

    let sent = alerter.alert(&store, Level::Warn, CODE, "plain message", None, t);
    let body = handle.join().unwrap();

    assert!(sent);
    assert!(!body.contains(SECRET), "posted body leaked: {body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(!v["code"].as_str().unwrap().is_empty());
    let stored = store.alerts().unwrap();
    assert_eq!(stored.len(), 1);
    assert!(!stored[0].1.is_empty());
    assert!(!stored[0].1.contains(SECRET), "stored code leaked");
}

// R-522: the line written to stderr does not contain the secret from the code.
// The alert is raised in a child process so its stderr can be captured.
#[test]
fn secret_in_code_is_absent_from_stderr() {
    if std::env::var(CHILD_ENV).is_ok() {
        let store = Store::open_in_memory().unwrap();
        let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
        let alerter = Alerter::new(None, vec![SECRET.to_string()]);
        alerter.alert(&store, Level::Warn, CODE, "plain message", None, t);
        return;
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "secret_in_code_is_absent_from_stderr",
            "--exact",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("plain message"),
        "alert line missing: {stderr}"
    );
    assert!(!stderr.contains(SECRET), "stderr leaked: {stderr}");
}
