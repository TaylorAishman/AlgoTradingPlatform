mod common;

use std::sync::Arc;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};

use common::{Fake, Listener};

/// Nothing listens here, so a request sent to the trading host fails at once.
const DEAD_TRADING: &str = "http://127.0.0.1:1";

fn client(trading: &str, data: &str) -> HttpClient {
    let fake = Fake::new();
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints = Endpoints::with_overrides(Some(trading), Some(data)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    )
}

// R-082: get_data goes to the data host, with the path and query given and both auth headers.
#[test]
fn get_data_requests_the_data_host_with_auth_headers() {
    let l = Listener::serve(vec![(200, r#"{"bars":{}}"#.to_string())]);
    let http = client(DEAD_TRADING, &l.url);
    let body = http.get_data("/v2/stocks/bars?symbols=AAPL&timeframe=1Day");
    assert_eq!(body.expect("served by the data host"), r#"{"bars":{}}"#);
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/stocks/bars");
    assert_eq!(seen[0].query(), "symbols=AAPL&timeframe=1Day");
    assert_eq!(seen[0].header("APCA-API-KEY-ID"), Some("FAKEKEYID"));
    assert_eq!(seen[0].header("APCA-API-SECRET-KEY"), Some("FAKESECRET"));
}

// R-082: get_trading still goes to the trading host, not the data host.
#[test]
fn get_trading_requests_the_trading_host() {
    let l = Listener::serve(vec![(200, "{}".to_string())]);
    let http = client(&l.url, DEAD_TRADING);
    let body = http.get_trading("/v2/clock");
    assert_eq!(body.expect("served by the trading host"), "{}");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path(), "/v2/clock");
}

// R-082: get_data shares the 4xx report of the common request path.
#[test]
fn get_data_reports_a_4xx_from_the_data_host() {
    let l = Listener::serve(vec![(403, r#"{"message":"no"}"#.to_string())]);
    let http = client(DEAD_TRADING, &l.url);
    let err = http.get_data("/v2/stocks/bars").unwrap_err();
    assert!(
        matches!(err, AlpacaError::Rejected { status: 403, .. }),
        "{err:?}"
    );
    assert_eq!(l.finish().len(), 1);
}
