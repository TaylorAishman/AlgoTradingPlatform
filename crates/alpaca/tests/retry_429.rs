mod common;

use std::sync::Arc;
use std::time::Duration;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::reference::clock;

use common::{Fake, Listener};

const CLOCK_BODY: &str = r#"{"timestamp":"2025-06-24T14:15:22-04:00","is_open":true,"next_open":"2025-06-25T09:30:00-04:00","next_close":"2025-06-24T16:00:00-04:00"}"#;

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

fn secs(v: &[u64]) -> Vec<Duration> {
    v.iter().map(|s| Duration::from_secs(*s)).collect()
}

// R-085: two 429 replies then 200 succeeds after sleeps of 15 s then 30 s.
#[test]
fn two_429s_then_200_succeeds_after_15_and_30_seconds() {
    let l = Listener::serve(vec![
        (429, String::new()),
        (429, String::new()),
        (200, CLOCK_BODY.to_string()),
    ]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let c = clock(&http).expect("clock succeeds after retries");
    assert!(c.is_open);
    assert_eq!(l.finish().len(), 3);
    assert_eq!(fake.sleeps(), secs(&[15, 30]));
}

// R-085: a permanent 429 is reported as RateLimited after 5 attempts, sleeping 15, 30, 45, 60 s.
#[test]
fn permanent_429_is_reported_after_five_attempts() {
    let l = Listener::serve((0..5).map(|_| (429, String::new())).collect());
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let err = clock(&http).unwrap_err();
    assert_eq!(err, AlpacaError::RateLimited);
    assert_eq!(fake.sleeps(), secs(&[15, 30, 45, 60]));
    assert_eq!(l.finish().len(), 5);
}
