mod common;

use std::sync::Arc;

use pt_alpaca::account::account;
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use rust_decimal::Decimal;

use common::{Fake, Listener};

const ACCOUNT_BODY: &str = include_str!("fixtures/account.json");

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

// R-093: string decimals are parsed exactly and unknown fields (id) are ignored.
#[test]
fn account_is_parsed_from_string_decimals() {
    let l = Listener::serve(vec![(200, ACCOUNT_BODY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let a = account(&http).expect("account");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/account");
    assert_eq!(seen[0].header("APCA-API-KEY-ID"), Some("FAKEKEYID"));
    assert_eq!(a.status, "ACTIVE");
    assert_eq!(a.cash, Decimal::new(100000, 0));
    assert_eq!(a.equity, Decimal::new(10000050, 2));
    assert_eq!(a.last_equity, Decimal::new(99990, 0));
    assert_eq!(a.buying_power, Decimal::new(200000, 0));
    assert_eq!(a.multiplier, Decimal::new(2, 0));
    assert!(a.shorting_enabled);
    assert!(!a.trading_blocked);
    assert!(!a.account_blocked);
}

// R-093: a decimal that is not a number is a parse error, not a zero.
#[test]
fn account_with_unparseable_cash_is_a_parse_error() {
    let body = ACCOUNT_BODY.replace("\"cash\":\"100000\"", "\"cash\":\"lots\"");
    let l = Listener::serve(vec![(200, body)]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let r = account(&http);
    l.finish();
    assert!(matches!(r, Err(AlpacaError::Parse(_))), "got {r:?}");
}

// R-093: a missing required field is a parse error.
#[test]
fn account_missing_a_field_is_a_parse_error() {
    let l = Listener::serve(vec![(200, r#"{"status":"ACTIVE"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let r = account(&http);
    l.finish();
    assert!(matches!(r, Err(AlpacaError::Parse(_))), "got {r:?}");
}
