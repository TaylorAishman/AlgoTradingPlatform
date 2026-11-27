//! Tests that a redirecting webhook counts as a failure and is not followed (R-352).

use chrono::{TimeZone, Utc};
use pt_ops::alert::{Alerter, Level};
use pt_store::db::Store;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// A loopback listener that counts requests and answers each with `respond`.
struct Listener {
    url: String,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    handle: thread::JoinHandle<()>,
}

impl Listener {
    fn start(location: Option<String>, status: u16) -> Self {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/hook", server.server_addr().to_ip().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (c, s) = (count.clone(), stop.clone());
        let handle = thread::spawn(move || {
            let mut idle_after_stop = 0;
            while idle_after_stop < 2 {
                match server.recv_timeout(Duration::from_millis(300)).unwrap() {
                    Some(req) => {
                        c.fetch_add(1, Ordering::SeqCst);
                        let mut resp = tiny_http::Response::empty(status);
                        if let Some(loc) = &location {
                            resp = resp.with_header(
                                tiny_http::Header::from_bytes(&b"Location"[..], loc.as_bytes())
                                    .unwrap(),
                            );
                        }
                        req.respond(resp).unwrap();
                    }
                    None if s.load(Ordering::SeqCst) => idle_after_stop += 1,
                    None => {}
                }
            }
        });
        Self {
            url,
            count,
            stop,
            handle,
        }
    }

    /// Stop the listener and return how many requests it received.
    fn finish(self) -> usize {
        self.stop.store(true, Ordering::SeqCst);
        self.handle.join().unwrap();
        self.count.load(Ordering::SeqCst)
    }
}

fn check_redirect(status: u16) {
    let second = Listener::start(None, 200);
    let first = Listener::start(Some(second.url.clone()), status);
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(Some(first.url.clone()), vec![]);

    let sent = alerter.alert(&store, Level::Critical, "halt_set", "msg", Some(2), t);
    let first_count = first.finish();
    let second_count = second.finish();

    assert!(!sent, "status {status} must count as a failure");
    assert_eq!(first_count, 1, "status {status}: first listener requests");
    assert_eq!(second_count, 0, "status {status}: redirect was followed");
    assert_eq!(
        store.alerts().unwrap(),
        vec![(
            "critical".to_string(),
            "halt_set".to_string(),
            "msg".to_string(),
            false
        )]
    );
}

// R-352: 301 is a failure and is not followed.
#[test]
fn redirect_301_is_a_failure_and_not_followed() {
    check_redirect(301);
}

// R-352: 302 is a failure and is not followed.
#[test]
fn redirect_302_is_a_failure_and_not_followed() {
    check_redirect(302);
}

// R-352: 307 is a failure and is not followed.
#[test]
fn redirect_307_is_a_failure_and_not_followed() {
    check_redirect(307);
}

// R-352: 308 is a failure and is not followed.
#[test]
fn redirect_308_is_a_failure_and_not_followed() {
    check_redirect(308);
}
