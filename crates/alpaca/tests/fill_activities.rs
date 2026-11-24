mod common;

use std::str::FromStr;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use pt_alpaca::activities::fill_activities;
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use rust_decimal::Decimal;

use common::{Fake, Listener};

const ACTIVITIES: &str = include_str!("fixtures/activities.json");

fn client(url: &str, fake: &Arc<Fake>) -> HttpClient {
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone());
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

/// A full page of 100 fills, ids `20261201::p0` to `20261201::p99`.
fn full_page() -> String {
    let items: Vec<String> = (0..100)
        .map(|i| {
            format!(
                r#"{{"id":"20261201::p{i}","activity_type":"FILL","order_id":"b{i}","symbol":"SPY","side":"buy","qty":"1","price":"100","transaction_time":"2026-12-01T14:00:00Z","type":"fill"}}"#
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

// R-099: the request is GET /v2/account/activities/FILL with the fixed paging parameters.
#[test]
fn sends_get_fill_activities_with_direction_page_size_and_after() {
    let l = Listener::serve(vec![(200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 13, 0, 0).unwrap();
    let result = fill_activities(&http, t);
    assert_eq!(result, Ok(vec![]));
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/account/activities/FILL");
    let q = seen[0].query();
    assert_eq!(param(q, "direction").as_deref(), Some("asc"));
    assert_eq!(param(q, "page_size").as_deref(), Some("100"));
    assert_eq!(param(q, "after").as_deref(), Some("2026-12-01T13:00:00Z"));
    assert_eq!(param(q, "page_token"), None);
}

// R-099: a short page is the last; its items parse to the stated values.
#[test]
fn short_page_stops_after_one_request_and_parses_fills() {
    let l = Listener::serve(vec![(200, ACTIVITIES.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let fills = fill_activities(&http, t).expect("ok");
    assert_eq!(l.finish().len(), 1);
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].id, "20261201::a1");
    assert_eq!(fills[0].order_id, "b1");
    assert_eq!(fills[0].symbol, "SPY");
    assert_eq!(fills[0].side, "buy");
    assert_eq!(fills[0].qty, 4);
    assert_eq!(fills[0].price, Decimal::from_str("472.11").unwrap());
    assert_eq!(
        fills[0].transaction_time,
        Utc.with_ymd_and_hms(2026, 12, 1, 14, 30, 1).unwrap()
    );
    assert_eq!(fills[1].id, "20261201::a2");
    assert_eq!(fills[1].qty, 6);
    assert_eq!(fills[1].price, Decimal::from_str("472.5").unwrap());
}

// R-099: 100 then 2 items gives 102 fills in two requests, the second with page_token set.
#[test]
fn full_page_then_short_page_returns_all_in_two_requests() {
    let l = Listener::serve(vec![(200, full_page()), (200, ACTIVITIES.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let fills = fill_activities(&http, t).expect("ok");
    let seen = l.finish();
    assert_eq!(seen.len(), 2);
    assert_eq!(fills.len(), 102);
    assert_eq!(fills[0].id, "20261201::p0");
    assert_eq!(fills[99].id, "20261201::p99");
    assert_eq!(fills[100].id, "20261201::a1");
    assert_eq!(fills[101].id, "20261201::a2");
    assert_eq!(param(seen[0].query(), "page_token"), None);
    assert_eq!(
        param(seen[1].query(), "page_token").as_deref(),
        Some("20261201::p99")
    );
    for s in &seen {
        assert_eq!(s.path(), "/v2/account/activities/FILL");
        assert_eq!(param(s.query(), "direction").as_deref(), Some("asc"));
        assert_eq!(param(s.query(), "page_size").as_deref(), Some("100"));
    }
}

// R-099 (boundary): exactly 100 items forces one more request; an empty next page ends it.
#[test]
fn full_page_then_empty_page_returns_100_in_two_requests() {
    let l = Listener::serve(vec![(200, full_page()), (200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let fills = fill_activities(&http, t).expect("ok");
    assert_eq!(l.finish().len(), 2);
    assert_eq!(fills.len(), 100);
}

// R-099 (boundary): a malformed body is a parse error.
#[test]
fn malformed_body_is_parse_error() {
    let l = Listener::serve(vec![(200, r#"[{"id":"x"}]"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let err = fill_activities(&http, t).unwrap_err();
    assert!(matches!(err, AlpacaError::Parse(_)), "got {err:?}");
    l.finish();
}

// R-099 (boundary): a 403 is rejected after one request.
#[test]
fn forbidden_is_rejected() {
    let l = Listener::serve(vec![(403, r#"{"message":"forbidden"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    let err = fill_activities(&http, t).unwrap_err();
    assert!(
        matches!(err, AlpacaError::Rejected { status: 403, .. }),
        "got {err:?}"
    );
    assert_eq!(l.finish().len(), 1);
}

// R-099: reserved characters in the page token are percent-encoded in the query.
#[test]
fn page_token_is_percent_encoded() {
    let items: Vec<String> = (0..100)
        .map(|i| {
            format!(
                r#"{{"id":"a&b={i}","activity_type":"FILL","order_id":"b{i}","symbol":"SPY","side":"buy","qty":"1","price":"100","transaction_time":"2026-12-01T14:00:00Z","type":"fill"}}"#
            )
        })
        .collect();
    let page = format!("[{}]", items.join(","));
    let l = Listener::serve(vec![(200, page), (200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let t = Utc.with_ymd_and_hms(2026, 12, 1, 0, 0, 0).unwrap();
    fill_activities(&http, t).expect("ok");
    let seen = l.finish();
    assert!(
        seen[1].query().ends_with("&page_token=a%26b%3D99"),
        "got {}",
        seen[1].query()
    );
}
