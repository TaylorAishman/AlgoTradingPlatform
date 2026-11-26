mod common;

use std::sync::Arc;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::reference::clock;

use common::{Fake, Listener};

const KEY: &str = "FAKEKEYID";
const SECRET: &str = "FAKESECRET";

fn client(url: &str, fake: &Arc<Fake>) -> HttpClient {
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints = Endpoints::with_overrides(Some(url), Some(url)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new(KEY.to_string(), SECRET.to_string()),
        limiter,
        sleeper,
    )
}

// R-087: a 422 is reported once, with the server message, and nothing sleeps.
#[test]
fn status_422_is_rejected_with_message_after_one_request() {
    let l = Listener::serve(vec![(422, r#"{"message":"bad input"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let err = clock(&http).unwrap_err();
    let sleeps = fake.sleeps();
    match &err {
        AlpacaError::Rejected { status, message } => {
            assert_eq!(*status, 422);
            assert!(message.contains("bad input"), "message was {message}");
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
    assert!(sleeps.is_empty(), "slept: {sleeps:?}");
    assert_eq!(l.finish().len(), 1);
}

// R-087: the error's Display text contains neither credential value.
#[test]
fn rejected_display_does_not_leak_credentials() {
    let l = Listener::serve(vec![(422, r#"{"message":"bad input"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let err = clock(&http).unwrap_err();
    let text = err.to_string();
    assert!(!text.contains(KEY), "leaked key id: {text}");
    assert!(!text.contains(SECRET), "leaked secret: {text}");
    assert!(text.contains("bad input"), "text was {text}");
    assert_eq!(l.finish().len(), 1);
}

// R-087: other 4xx statuses (400, 404) are likewise not retried.
#[test]
fn other_4xx_statuses_are_not_retried() {
    for status in [400u16, 404] {
        let l = Listener::serve(vec![(status, r#"{"message":"nope"}"#.to_string())]);
        let fake = Fake::new();
        let http = client(&l.url, &fake);
        let err = clock(&http).unwrap_err();
        let sleeps = fake.sleeps();
        match &err {
            AlpacaError::Rejected { status: s, message } => {
                assert_eq!(*s, status);
                assert!(message.contains("nope"), "message was {message}");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
        assert!(sleeps.is_empty(), "slept: {sleeps:?}");
        assert_eq!(l.finish().len(), 1);
    }
}
