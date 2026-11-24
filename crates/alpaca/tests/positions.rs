mod common;

use std::sync::Arc;

use pt_alpaca::account::positions;
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};

use common::{Fake, Listener};

const POSITIONS_BODY: &str = include_str!("fixtures/positions.json");

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

// R-094: quantities are signed whole shares; a short side makes the quantity negative.
#[test]
fn positions_are_signed_by_side() {
    let l = Listener::serve(vec![(200, POSITIONS_BODY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let p = positions(&http).expect("positions");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/positions");
    let got: Vec<(&str, i64)> = p.iter().map(|w| (w.symbol.as_str(), w.qty)).collect();
    assert_eq!(got, vec![("SPY", 10), ("TLT", -5), ("GLD", -7)]);
}

// R-094: an empty account has no positions.
#[test]
fn no_positions_is_an_empty_list() {
    let l = Listener::serve(vec![(200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let p = positions(&http).expect("positions");
    l.finish();
    assert!(p.is_empty());
}

// R-094: a fractional quantity is a parse error that says so.
#[test]
fn fractional_quantity_is_a_parse_error() {
    let body = r#"[{"symbol":"SPY","qty":"1.5","side":"long"}]"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let r = positions(&http);
    l.finish();
    match r {
        Err(AlpacaError::Parse(m)) => assert!(m.contains("fractional"), "message: {m}"),
        other => panic!("expected a parse error, got {other:?}"),
    }
}

// R-094: a quantity that is not a number is a parse error.
#[test]
fn non_numeric_quantity_is_a_parse_error() {
    let body = r#"[{"symbol":"SPY","qty":"many","side":"long"}]"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let r = positions(&http);
    l.finish();
    assert!(matches!(r, Err(AlpacaError::Parse(_))), "got {r:?}");
}

// R-094: a position without a side cannot be signed, so it is a parse error.
#[test]
fn missing_side_is_a_parse_error() {
    let body = r#"[{"symbol":"SPY","qty":"3"}]"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let r = positions(&http);
    l.finish();
    assert!(matches!(r, Err(AlpacaError::Parse(_))), "got {r:?}");
}
