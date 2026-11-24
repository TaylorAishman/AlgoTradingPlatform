//! Shared helpers: a loopback listener that records requests, and a fake clock and sleeper.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use pt_alpaca::limiter::{MonoClock, Sleeper};

/// One request as the listener saw it.
#[derive(Debug, Clone)]
pub struct Seen {
    pub method: String,
    /// Path and query, as sent.
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Seen {
    /// Value of a header, matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn path(&self) -> &str {
        self.url.split('?').next().unwrap_or("")
    }

    pub fn query(&self) -> &str {
        self.url.split_once('?').map_or("", |(_, q)| q)
    }
}

/// A loopback listener that answers a fixed list of (status, body) replies in order.
pub struct Listener {
    pub url: String,
    handle: JoinHandle<Vec<Seen>>,
}

impl Listener {
    /// Serve exactly `replies.len()` requests, then stop.
    pub fn serve(replies: Vec<(u16, String)>) -> Listener {
        let server = tiny_http::Server::http("127.0.0.1:0").expect("bind loopback");
        let port = server.server_addr().to_ip().expect("ip address").port();
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (status, body) in replies {
                let mut req = server.recv().expect("receive");
                let mut text = String::new();
                std::io::Read::read_to_string(req.as_reader(), &mut text).expect("read body");
                seen.push(Seen {
                    method: req.method().to_string(),
                    url: req.url().to_string(),
                    headers: req
                        .headers()
                        .iter()
                        .map(|h| (h.field.to_string(), h.value.to_string()))
                        .collect(),
                    body: text,
                });
                let resp = tiny_http::Response::from_string(body)
                    .with_status_code(status)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .expect("header"),
                    );
                req.respond(resp).expect("respond");
            }
            seen
        });
        Listener {
            url: format!("http://127.0.0.1:{port}"),
            handle,
        }
    }

    /// Wait for the listener to finish and return what it saw.
    pub fn finish(self) -> Vec<Seen> {
        self.handle.join().expect("listener thread")
    }
}

/// A fake clock whose sleeper advances the clock and records each sleep.
#[derive(Default)]
pub struct Fake {
    now: Mutex<Duration>,
    sleeps: Mutex<Vec<Duration>>,
}

impl Fake {
    pub fn new() -> Arc<Fake> {
        Arc::new(Fake::default())
    }

    pub fn sleeps(&self) -> Vec<Duration> {
        self.sleeps.lock().unwrap().clone()
    }
}

impl MonoClock for Fake {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}

impl Sleeper for Fake {
    fn sleep(&self, d: Duration) {
        self.sleeps.lock().unwrap().push(d);
        *self.now.lock().unwrap() += d;
    }
}
