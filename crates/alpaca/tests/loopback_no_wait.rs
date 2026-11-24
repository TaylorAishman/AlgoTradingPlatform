//! R-103, architecture 5.5: the test client has no rate limit and never really sleeps.

mod common;

use std::time::{Duration, Instant};

use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::loopback_client;

use common::Listener;

/// Far below the shortest real wait these tests would hit (15 s), far above
/// what a handful of loopback requests take.
const BOUND: Duration = Duration::from_secs(5);

// A 429 costs a real client 15 s before the retry.
#[test]
fn a_retry_after_429_does_not_wait() {
    let l = Listener::serve(vec![(429, String::new()), (200, "{}".to_string())]);
    let http = loopback_client(&l.url).expect("loopback");
    let started = Instant::now();
    let body = http.get_trading("/v2/clock");
    let took = started.elapsed();
    assert_eq!(body.expect("the retry succeeds"), "{}");
    assert!(took < BOUND, "took {took:?}");
    assert_eq!(l.finish().len(), 2);
}

// Five 5xx replies cost a real client 1 + 2 + 4 + 8 = 15 s.
#[test]
fn exhausted_5xx_retries_do_not_wait() {
    let l = Listener::serve(vec![(500, String::new()); 5]);
    let http = loopback_client(&l.url).expect("loopback");
    let started = Instant::now();
    let err = http.get_trading("/v2/clock").unwrap_err();
    let took = started.elapsed();
    assert!(
        matches!(err, AlpacaError::Server { status: 500 }),
        "{err:?}"
    );
    assert!(took < BOUND, "took {took:?}");
    assert_eq!(l.finish().len(), 5);
}

// A 200 per minute limiter holds the 201st request for the rest of the minute.
#[test]
fn request_201_is_not_held_back() {
    let l = Listener::serve(vec![(200, "{}".to_string()); 201]);
    let http = loopback_client(&l.url).expect("loopback");
    let started = Instant::now();
    for _ in 0..201 {
        http.get_trading("/v2/clock").expect("ok");
    }
    let took = started.elapsed();
    assert!(took < BOUND, "took {took:?}");
    assert_eq!(l.finish().len(), 201);
}
