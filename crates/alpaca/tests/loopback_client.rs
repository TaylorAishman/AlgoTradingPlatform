mod common;

use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::loopback_client;
use pt_alpaca::reference::clock;

use common::Listener;

const CLOCK_BODY: &str = r#"{"timestamp":"2025-06-24T14:15:22-04:00","is_open":true,"next_open":"2025-06-25T09:30:00-04:00","next_close":"2025-06-24T16:00:00-04:00"}"#;

// R-103: a loopback URL is accepted.
#[test]
fn loopback_url_is_accepted() {
    assert!(loopback_client("http://127.0.0.1:1234").is_ok());
}

// R-103: the paper host is refused with EndpointNotAllowed.
#[test]
fn paper_host_is_refused() {
    let err = loopback_client("https://paper-api.alpaca.markets").unwrap_err();
    assert!(matches!(err, AlpacaError::EndpointNotAllowed(_)), "{err:?}");
}

// R-103: any other non-loopback host is refused too.
#[test]
fn other_remote_host_is_refused() {
    let err = loopback_client("https://example.com").unwrap_err();
    assert!(matches!(err, AlpacaError::EndpointNotAllowed(_)), "{err:?}");
}

// R-103: the returned client talks to the loopback URL and sends auth headers.
#[test]
fn loopback_client_reaches_the_loopback_listener() {
    let l = Listener::serve(vec![(200, CLOCK_BODY.to_string())]);
    let http = loopback_client(&l.url).expect("loopback");
    let c = clock(&http).expect("clock");
    let seen = l.finish();
    assert!(c.is_open);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path(), "/v2/clock");
    assert!(seen[0].header("APCA-API-KEY-ID").is_some());
    assert!(seen[0].header("APCA-API-SECRET-KEY").is_some());
}
