mod common;

use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::reference::clock;

use common::{Fake, Listener};

const CLOCK_BODY: &str = r#"{"timestamp":"2025-06-24T14:15:22-04:00","is_open":true,"next_open":"2025-06-25T09:30:00-04:00","next_close":"2025-06-24T16:00:00-04:00"}"#;

fn client(url: &str, key: &str, secret: &str) -> HttpClient {
    let fake = Fake::new();
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints = Endpoints::with_overrides(Some(url), Some(url)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new(key.to_string(), secret.to_string()),
        limiter,
        sleeper,
    )
}

fn at(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

// R-082: every request carries both auth headers with the given values.
#[test]
fn clock_request_carries_both_auth_headers() {
    let l = Listener::serve(vec![(200, CLOCK_BODY.to_string())]);
    let http = client(&l.url, "FAKEKEYID", "FAKESECRET");
    clock(&http).expect("clock");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/clock");
    assert_eq!(seen[0].header("APCA-API-KEY-ID"), Some("FAKEKEYID"));
    assert_eq!(seen[0].header("APCA-API-SECRET-KEY"), Some("FAKESECRET"));
}

// R-095: the clock is parsed with the instants preserved with their offsets.
#[test]
fn clock_is_parsed_with_offsets_preserved() {
    let l = Listener::serve(vec![(200, CLOCK_BODY.to_string())]);
    let http = client(&l.url, "FAKEKEYID", "FAKESECRET");
    let c = clock(&http).expect("clock");
    l.finish();
    assert!(c.is_open);
    assert_eq!(c.timestamp, at("2025-06-24T14:15:22-04:00"));
    assert_eq!(c.next_open, at("2025-06-25T09:30:00-04:00"));
    assert_eq!(c.next_close, at("2025-06-24T16:00:00-04:00"));
    assert_eq!(c.timestamp.offset().local_minus_utc(), -4 * 3600);
    assert_eq!(c.next_open.offset().local_minus_utc(), -4 * 3600);
}
