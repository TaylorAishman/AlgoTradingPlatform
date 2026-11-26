mod common;

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::orders_read::order_by_client_id;

use common::{Fake, Listener};

const ORDER: &str = include_str!("fixtures/order.json");

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

// R-097: the lookup is a GET to the by-client-id path with the id as a query parameter.
#[test]
fn lookup_sends_get_with_client_order_id_query() {
    let l = Listener::serve(vec![(200, ORDER.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = order_by_client_id(&http, "pt-x-1");
    assert!(result.is_ok(), "got {result:?}");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/orders:by_client_order_id");
    assert_eq!(seen[0].query(), "client_order_id=pt-x-1");
}

// R-097: a 200 body parses to the stated values.
#[test]
fn lookup_parses_the_order() {
    let l = Listener::serve(vec![(200, ORDER.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let order = order_by_client_id(&http, "pt-x-1")
        .expect("ok")
        .expect("some order");
    assert_eq!(order.id, "b1");
    assert_eq!(order.client_order_id, "pt-x-1");
    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.side, "buy");
    assert_eq!(order.qty, 10);
    assert_eq!(order.filled_qty, 4);
    assert_eq!(order.status, "partially_filled");
    assert_eq!(
        order.submitted_at,
        Utc.with_ymd_and_hms(2026, 12, 1, 13, 0, 0).unwrap()
    );
    l.finish();
}

// R-097: a 404 means the order does not exist, which is Ok(None), after one request.
#[test]
fn lookup_404_is_none() {
    let l = Listener::serve(vec![(404, r#"{"message":"order not found"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = order_by_client_id(&http, "pt-x-9");
    assert_eq!(result, Ok(None));
    assert_eq!(l.finish().len(), 1);
}

// R-097 (boundary): other 4xx answers are still errors, not "not found".
#[test]
fn lookup_403_is_rejected() {
    let l = Listener::serve(vec![(403, r#"{"message":"forbidden"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let err = order_by_client_id(&http, "pt-x-1").unwrap_err();
    assert!(
        matches!(err, AlpacaError::Rejected { status: 403, .. }),
        "got {err:?}"
    );
    l.finish();
}

// R-097 (boundary): a malformed body is a parse error.
#[test]
fn lookup_malformed_body_is_parse_error() {
    let l = Listener::serve(vec![(200, r#"{"id":"b1"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let err = order_by_client_id(&http, "pt-x-1").unwrap_err();
    assert!(matches!(err, AlpacaError::Parse(_)), "got {err:?}");
    l.finish();
}

// R-097: reserved characters in the id are percent-encoded in the query.
#[test]
fn lookup_percent_encodes_the_client_order_id() {
    let l = Listener::serve(vec![(404, r#"{"message":"not found"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = order_by_client_id(&http, "a&b=c");
    assert_eq!(result, Ok(None));
    let seen = l.finish();
    assert_eq!(seen[0].query(), "client_order_id=a%26b%3Dc");
}
