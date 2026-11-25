//! Tests that a failing webhook never fails the caller (R-352).

use chrono::{TimeZone, Utc};
use pt_ops::alert::{Alerter, Level};
use pt_store::db::Store;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// R-352: a closed loopback port gives false, no panic, and a stored alert
// with delivered == false.
#[test]
fn closed_port_returns_false_and_stores_undelivered() {
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(Some(format!("http://127.0.0.1:{port}/hook")), vec![]);

    let sent = alerter.alert(&store, Level::Warn, "feed_down", "msg", None, t);

    assert!(!sent);
    assert_eq!(
        store.alerts().unwrap(),
        vec![(
            "warn".to_string(),
            "feed_down".to_string(),
            "msg".to_string(),
            false
        )]
    );
}

// R-352: a listener answering 500 gives false, exactly one attempt, and a
// stored alert with delivered == false.
#[test]
fn server_error_returns_false_after_one_attempt_and_stores_undelivered() {
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
                    req.respond(tiny_http::Response::empty(500)).unwrap();
                }
                None if s.load(Ordering::SeqCst) => idle_after_stop += 1,
                None => {}
            }
        }
    });

    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(Some(url), vec![]);
    let sent = alerter.alert(&store, Level::Critical, "halt_set", "msg", Some(2), t);
    stop.store(true, Ordering::SeqCst);
    handle.join().unwrap();

    assert!(!sent);
    assert_eq!(count.load(Ordering::SeqCst), 1);
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

// R-350, R-352: a listener that accepts and never answers must not block the
// alert forever; it returns false in bounded time and stores it undelivered.
#[test]
fn unresponsive_listener_times_out_and_stores_undelivered() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let done = Arc::new(AtomicBool::new(false));
    let d = done.clone();
    let handle = thread::spawn(move || {
        // Hold the accepted connection open without replying until told to stop.
        let conn = listener.accept().unwrap();
        while !d.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }
        drop(conn);
    });

    let store = Store::open_in_memory().unwrap();
    let t = Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 0).unwrap();
    let alerter = Alerter::new(Some(format!("http://127.0.0.1:{port}/hook")), vec![]);
    let started = std::time::Instant::now();
    let sent = alerter.alert(&store, Level::Warn, "feed_down", "msg", None, t);
    let elapsed = started.elapsed();
    done.store(true, Ordering::SeqCst);
    handle.join().unwrap();

    assert!(!sent);
    assert!(elapsed < Duration::from_secs(15), "took {elapsed:?}");
    assert_eq!(
        store.alerts().unwrap(),
        vec![(
            "warn".to_string(),
            "feed_down".to_string(),
            "msg".to_string(),
            false
        )]
    );
}
