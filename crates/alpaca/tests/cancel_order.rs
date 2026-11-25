mod common;

use std::sync::Arc;
use std::time::Duration;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::orders_write::cancel_order;

use common::{Fake, Listener};

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

fn secs(list: &[u64]) -> Vec<Duration> {
    list.iter().map(|s| Duration::from_secs(*s)).collect()
}

// R-102: a 204 with no body gives Ok(()) after one DELETE to /v2/orders/b1.
// R-082: the DELETE carries both auth headers.
#[test]
fn cancel_sends_delete_with_auth_and_accepts_204() {
    let l = Listener::serve(vec![(204, String::new())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "b1");
    assert_eq!(result, Ok(()));
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "DELETE");
    assert_eq!(seen[0].path(), "/v2/orders/b1");
    assert_eq!(seen[0].header("APCA-API-KEY-ID"), Some("FAKEKEYID"));
    assert_eq!(seen[0].header("APCA-API-SECRET-KEY"), Some("FAKESECRET"));
}

// R-102: any 2xx is success and its body is ignored.
#[test]
fn cancel_ignores_the_body_of_a_200() {
    let l = Listener::serve(vec![(200, "not json at all".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "b2");
    assert_eq!(result, Ok(()));
    let seen = l.finish();
    assert_eq!(seen[0].path(), "/v2/orders/b2");
}

// R-102: a 422 is Rejected, after one request and no sleep.
#[test]
fn cancel_422_is_rejected_without_retry() {
    let l = Listener::serve(vec![(
        422,
        r#"{"message":"order is not cancelable"}"#.to_string(),
    )]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "b1");
    assert!(fake.sleeps().is_empty());
    match result {
        Err(AlpacaError::Rejected { status, message }) => {
            assert_eq!(status, 422);
            assert!(message.contains("not cancelable"), "got {message}");
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
    assert_eq!(l.finish().len(), 1);
}

// R-102 (and R-087): an unknown order, 404, is Rejected and is not retried.
#[test]
fn cancel_404_is_rejected_without_retry() {
    let l = Listener::serve(vec![(404, r#"{"message":"order not found"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "gone");
    assert!(fake.sleeps().is_empty());
    assert!(
        matches!(result, Err(AlpacaError::Rejected { status: 404, .. })),
        "got {result:?}"
    );
    assert_eq!(l.finish().len(), 1);
}

// R-086: a 503 then a 204 succeeds after one sleep of 1 s.
#[test]
fn cancel_retries_a_503_once_then_succeeds() {
    let l = Listener::serve(vec![(503, String::new()), (204, String::new())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "b1");
    assert_eq!(result, Ok(()));
    assert_eq!(fake.sleeps(), secs(&[1]));
    let seen = l.finish();
    assert_eq!(seen.len(), 2);
    assert!(seen.iter().all(|s| s.method == "DELETE"));
}

// R-086: five 500 replies give Server after five requests and sleeps 1, 2, 4, 8 s.
#[test]
fn cancel_reports_server_error_after_five_attempts() {
    let l = Listener::serve(vec![
        (500, String::new()),
        (500, String::new()),
        (500, String::new()),
        (500, String::new()),
        (500, String::new()),
    ]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "b1");
    assert_eq!(result, Err(AlpacaError::Server { status: 500 }));
    assert_eq!(fake.sleeps(), secs(&[1, 2, 4, 8]));
    assert_eq!(l.finish().len(), 5);
}

// R-085: a 429 to a DELETE is retried after 15 s, as for any call.
#[test]
fn cancel_retries_a_429_after_fifteen_seconds() {
    let l = Listener::serve(vec![(429, String::new()), (204, String::new())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = cancel_order(&http, "b1");
    assert_eq!(result, Ok(()));
    assert_eq!(fake.sleeps(), secs(&[15]));
    assert_eq!(l.finish().len(), 2);
}
