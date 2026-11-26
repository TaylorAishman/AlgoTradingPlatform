mod common;

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::orders_read::orders_since;

use common::{Fake, Listener};

const ORDERS: &str = include_str!("fixtures/orders.json");

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

/// The value of a query parameter, with an encoded colon decoded.
fn param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.replace("%3A", ":").replace("%3a", ":"))
    })
}

/// A full page of 500 orders, the last submitted at 2026-12-02T09:08:19Z.
fn full_page() -> String {
    let items: Vec<String> = (0..500)
        .map(|i| {
            let secs = i % 60;
            let mins = (i / 60) % 60;
            format!(
                r#"{{"id":"f{i}","client_order_id":"pt-f-{i}","symbol":"SPY","side":"buy","qty":"1","filled_qty":"1","status":"filled","submitted_at":"2026-12-02T09:{mins:02}:{secs:02}Z"}}"#
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

// R-098: the request is GET /v2/orders with the fixed paging parameters and the instant.
#[test]
fn sends_get_orders_with_status_direction_limit_and_after() {
    let l = Listener::serve(vec![(200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 13, 0, 0).unwrap();
    let result = orders_since(&http, t);
    assert_eq!(result, Ok(vec![]));
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/orders");
    let q = seen[0].query();
    assert_eq!(param(q, "status").as_deref(), Some("all"));
    assert_eq!(param(q, "direction").as_deref(), Some("asc"));
    assert_eq!(param(q, "limit").as_deref(), Some("500"));
    assert_eq!(param(q, "after").as_deref(), Some("2026-12-01T13:00:00Z"));
}

// R-098: a page with fewer than 500 orders is the last; its orders parse to the stated values.
#[test]
fn short_page_stops_after_one_request_and_parses_orders() {
    let l = Listener::serve(vec![(200, ORDERS.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let orders = orders_since(&http, t).expect("ok");
    assert_eq!(l.finish().len(), 1);
    assert_eq!(orders.len(), 3);
    assert_eq!(orders[0].id, "c1");
    assert_eq!(orders[0].client_order_id, "pt-x-1");
    assert_eq!(orders[0].symbol, "SPY");
    assert_eq!(orders[0].side, "buy");
    assert_eq!(orders[0].qty, 10);
    assert_eq!(orders[0].filled_qty, 10);
    assert_eq!(orders[0].status, "filled");
    assert_eq!(orders[2].id, "c3");
    assert_eq!(orders[2].qty, 7);
    assert_eq!(orders[2].filled_qty, 3);
    assert_eq!(
        orders[2].submitted_at,
        Utc.with_ymd_and_hms(2026, 12, 2, 15, 30, 0).unwrap()
    );
}

// R-098: 500 then 3 orders gives 503 orders in two requests, the second paged by the last submitted_at.
#[test]
fn full_page_then_short_page_returns_all_in_two_requests() {
    let l = Listener::serve(vec![(200, full_page()), (200, ORDERS.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let orders = orders_since(&http, t).expect("ok");
    let seen = l.finish();
    assert_eq!(seen.len(), 2);
    assert_eq!(orders.len(), 503);
    assert_eq!(orders[0].id, "f0");
    assert_eq!(orders[499].id, "f499");
    assert_eq!(orders[500].id, "c1");
    assert_eq!(orders[502].id, "c3");
    assert_eq!(
        param(seen[0].query(), "after").as_deref(),
        Some("2026-12-01T00:00:00Z")
    );
    // Order 499 is the 500th: minute (499 / 60) % 60 = 8, second 499 % 60 = 19.
    assert_eq!(
        param(seen[1].query(), "after").as_deref(),
        Some("2026-12-02T09:08:19Z")
    );
    for s in &seen {
        assert_eq!(s.path(), "/v2/orders");
        assert_eq!(param(s.query(), "limit").as_deref(), Some("500"));
        assert_eq!(param(s.query(), "status").as_deref(), Some("all"));
        assert_eq!(param(s.query(), "direction").as_deref(), Some("asc"));
    }
}

// R-098 (boundary): exactly 500 orders forces one more request; an empty next page ends it.
#[test]
fn full_page_then_empty_page_returns_500_in_two_requests() {
    let l = Listener::serve(vec![(200, full_page()), (200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let orders = orders_since(&http, t).expect("ok");
    assert_eq!(l.finish().len(), 2);
    assert_eq!(orders.len(), 500);
}

// R-098 (boundary): a malformed body is a parse error.
#[test]
fn malformed_body_is_parse_error() {
    let l = Listener::serve(vec![(200, r#"[{"id":"c1"}]"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let err = orders_since(&http, t).unwrap_err();
    assert!(matches!(err, AlpacaError::Parse(_)), "got {err:?}");
    l.finish();
}

// R-098 (boundary): a 403 is rejected after one request.
#[test]
fn forbidden_is_rejected() {
    let l = Listener::serve(vec![(403, r#"{"message":"forbidden"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let err = orders_since(&http, t).unwrap_err();
    assert!(
        matches!(err, AlpacaError::Rejected { status: 403, .. }),
        "got {err:?}"
    );
    assert_eq!(l.finish().len(), 1);
}

/// A full page of 500 orders whose last `submitted_at` has microseconds.
fn fractional_page() -> String {
    let items: Vec<String> = (0..500)
        .map(|i| {
            let stamp = if i == 499 {
                "2026-12-02T09:08:19.942282Z".to_string()
            } else {
                format!("2026-12-02T09:{:02}:{:02}Z", (i / 60) % 60, i % 60)
            };
            format!(
                r#"{{"id":"g{i}","client_order_id":"pt-g-{i}","symbol":"SPY","side":"buy","qty":"1","filled_qty":"1","status":"filled","submitted_at":"{stamp}"}}"#
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

// R-098: the cursor keeps sub-second precision, so no order repeats at a page boundary.
#[test]
fn cursor_keeps_fractional_seconds_and_no_order_repeats() {
    let l = Listener::serve(vec![(200, fractional_page()), (200, ORDERS.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let orders = orders_since(&http, t).expect("ok");
    let seen = l.finish();
    assert_eq!(seen.len(), 2);
    assert_eq!(
        param(seen[1].query(), "after").as_deref(),
        Some("2026-12-02T09:08:19.942282Z")
    );
    assert_eq!(orders.len(), 503);
    let mut ids: Vec<&str> = orders.iter().map(|o| o.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 503);
}

// R-098 (boundary): a full page that does not move the cursor forward is an error, not a loop.
#[test]
fn full_page_that_does_not_advance_the_cursor_is_an_error() {
    let l = Listener::serve(vec![(200, full_page())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    // The page's last order is 2026-12-02T09:08:19Z; start there.
    let t = Utc.with_ymd_and_hms(2026, 12, 2, 9, 8, 19).unwrap();
    let err = orders_since(&http, t).unwrap_err();
    assert!(matches!(err, AlpacaError::Parse(_)), "got {err:?}");
    assert_eq!(l.finish().len(), 1);
}
