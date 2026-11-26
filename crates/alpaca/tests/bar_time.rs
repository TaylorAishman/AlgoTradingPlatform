mod common;

use std::sync::Arc;

use chrono::NaiveDate;
use pt_alpaca::auth::Credentials;
use pt_alpaca::data::{daily_bars, WireBar};
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

fn fetch(body: &str) -> Result<Vec<WireBar>, AlpacaError> {
    let l = Listener::serve(vec![(200, body.to_string())]);
    let http = client(DEAD_TRADING, &l.url);
    let symbols = vec!["SPY".to_string()];
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 7, 5).unwrap();
    let result = daily_bars(&http, &symbols, start, end, "all", "sip");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    result
}

fn body_with(times: &[&str]) -> String {
    let bars: Vec<String> = times
        .iter()
        .map(|t| format!(r#"{{"t":"{t}","o":1.5,"h":2.5,"l":1.25,"c":2.0,"v":10}}"#))
        .collect();
    format!(
        r#"{{"bars":{{"SPY":[{}]}},"next_page_token":null}}"#,
        bars.join(",")
    )
}

fn assert_refused(time: &str) {
    let result = fetch(&body_with(&[time]));
    assert!(
        matches!(result, Err(AlpacaError::UnexpectedBarTime(_))),
        "{time}: {result:?}"
    );
}

// R-091: midnight New York in winter (05:00Z) and summer (04:00Z) parses to the UTC date.
#[test]
fn winter_and_summer_midnight_parse_to_their_dates() {
    let bars = fetch(&body_with(&[
        "2024-01-02T05:00:00Z",
        "2024-07-01T04:00:00Z",
    ]))
    .expect("midnight New York is accepted");
    assert_eq!(bars.len(), 2, "{bars:?}");
    assert_eq!(bars[0].date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    assert_eq!(bars[1].date, NaiveDate::from_ymd_opt(2024, 7, 1).unwrap());
}

// R-091: a bar at the market open, not midnight, is refused.
#[test]
fn bar_at_market_open_is_refused() {
    assert_refused("2024-01-02T14:30:00Z");
}

// R-091: a bar at UTC midnight is refused.
#[test]
fn bar_at_utc_midnight_is_refused() {
    assert_refused("2024-01-02T00:00:00Z");
}

// R-091: a time that is off by a minute or a second from 05:00Z / 04:00Z is refused.
#[test]
fn bar_off_midnight_by_a_minute_or_second_is_refused() {
    assert_refused("2024-01-02T05:01:00Z");
    assert_refused("2024-01-02T05:00:01Z");
    assert_refused("2024-07-01T04:30:00Z");
}

// R-091: one bad bar among good ones makes the whole call fail.
#[test]
fn one_bad_bar_among_good_ones_fails_the_call() {
    let body = body_with(&[
        "2024-01-02T05:00:00Z",
        "2024-01-03T14:30:00Z",
        "2024-01-04T05:00:00Z",
    ]);
    let result = fetch(&body);
    assert!(
        matches!(result, Err(AlpacaError::UnexpectedBarTime(_))),
        "{result:?}"
    );
}
