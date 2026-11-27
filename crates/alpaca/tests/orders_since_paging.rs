mod common;

use std::sync::Arc;

use chrono::{DateTime, Duration, SecondsFormat, TimeZone, Utc};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::orders_read::orders_since;

use common::Fake;

fn client(url: &str, fake: &Arc<Fake>) -> HttpClient {
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints = Endpoints::with_overrides(Some(url), Some(url)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    )
}

fn param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.replace("%3A", ":").replace("%3a", ":"))
    })
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap()
}

/// 501 orders, order i submitted i + 1 seconds after t0 (strictly increasing).
fn submitted(i: i64) -> DateTime<Utc> {
    t0() + Duration::seconds(i + 1)
}

fn order_json(i: i64) -> String {
    format!(
        r#"{{"id":"o{i}","client_order_id":"pt-o-{i}","symbol":"SPY","side":"buy","qty":"1","filled_qty":"1","status":"filled","submitted_at":"{}"}}"#,
        submitted(i).to_rfc3339_opts(SecondsFormat::Secs, true)
    )
}

/// Serves two requests like the broker: `after` is exclusive and at most 500 orders return.
/// Returns the `after` value of each request.
fn serve_exclusive_after() -> (String, std::thread::JoinHandle<Vec<String>>) {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind loopback");
    let port = server.server_addr().to_ip().expect("ip address").port();
    let handle = std::thread::spawn(move || {
        let mut afters = Vec::new();
        // Two requests are expected; a third would block, so the test fails by hanging only
        // if the client wrongly asks again, which the order-count assertions catch first.
        for _ in 0..2 {
            let req = server.recv().expect("receive");
            let url = req.url().to_string();
            let query = url.split_once('?').map_or("", |(_, q)| q).to_string();
            let after = param(&query, "after").expect("after present");
            let after_t = DateTime::parse_from_rfc3339(&after)
                .expect("after parses")
                .with_timezone(&Utc);
            let page: Vec<String> = (0..501)
                .filter(|i| submitted(*i) > after_t)
                .take(500)
                .map(order_json)
                .collect();
            afters.push(after);
            let resp = tiny_http::Response::from_string(format!("[{}]", page.join(",")))
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .expect("header"),
                );
            req.respond(resp).expect("respond");
        }
        afters
    });
    (format!("http://127.0.0.1:{port}"), handle)
}

// R-438: 501 orders over an exclusive-`after` server come back whole, each once, in two requests.
#[test]
fn pages_501_orders_with_exclusive_after_each_once_in_two_requests() {
    let (url, handle) = serve_exclusive_after();
    let fake = Fake::new();
    let http = client(&url, &fake);
    let orders = orders_since(&http, t0()).expect("ok");
    let afters = handle.join().expect("server thread");
    assert_eq!(orders.len(), 501);
    let ids: Vec<String> = orders.iter().map(|o| o.id.clone()).collect();
    let expected: Vec<String> = (0..501).map(|i| format!("o{i}")).collect();
    assert_eq!(ids, expected);
    assert_eq!(afters.len(), 2);
    assert_eq!(afters[0], "2026-12-01T00:00:00Z");
    // The second `after` is the submitted_at of the last order of the first page (order 499).
    assert_eq!(
        afters[1],
        submitted(499).to_rfc3339_opts(SecondsFormat::Secs, true)
    );
}
