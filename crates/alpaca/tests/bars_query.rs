mod common;

use std::sync::Arc;

use chrono::NaiveDate;
use pt_alpaca::auth::Credentials;
use pt_alpaca::data::daily_bars;
use pt_alpaca::endpoints::Endpoints;
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

/// Query pairs in the order sent, with a percent-encoded comma decoded.
fn pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (k.to_string(), v.replace("%2C", ",").replace("%2c", ","))
        })
        .collect()
}

fn value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

// R-088: daily_bars sends one GET to the data host's /v2/stocks/bars with the documented query.
#[test]
fn daily_bars_sends_the_documented_query_to_the_data_host() {
    let l = Listener::serve(vec![(
        200,
        r#"{"bars":{},"next_page_token":null}"#.to_string(),
    )]);
    let http = client(DEAD_TRADING, &l.url);
    let symbols = vec!["SPY".to_string(), "QQQ".to_string()];
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();

    let result = daily_bars(&http, &symbols, start, end, "all", "sip");
    assert!(result.is_ok(), "{result:?}");

    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/stocks/bars");
    let q = pairs(seen[0].query());
    assert_eq!(value(&q, "symbols"), Some("SPY,QQQ"));
    assert_eq!(value(&q, "timeframe"), Some("1Day"));
    assert_eq!(value(&q, "start"), Some("2024-01-02"));
    assert_eq!(value(&q, "end"), Some("2024-01-05"));
    assert_eq!(value(&q, "adjustment"), Some("all"));
    assert_eq!(value(&q, "feed"), Some("sip"));
    assert_eq!(value(&q, "limit"), Some("10000"));
    assert_eq!(value(&q, "sort"), Some("asc"));
    assert_eq!(value(&q, "page_token"), None);
    assert_eq!(q.len(), 8, "no other parameters: {q:?}");
}

// R-088: adjustment and feed are passed through, not fixed.
#[test]
fn daily_bars_passes_adjustment_and_feed_through() {
    let l = Listener::serve(vec![(
        200,
        r#"{"bars":{},"next_page_token":null}"#.to_string(),
    )]);
    let http = client(DEAD_TRADING, &l.url);
    let symbols = vec!["SPY".to_string()];
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();

    let result = daily_bars(&http, &symbols, start, end, "split", "iex");
    assert!(result.is_ok(), "{result:?}");

    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    let q = pairs(seen[0].query());
    assert_eq!(value(&q, "symbols"), Some("SPY"));
    assert_eq!(value(&q, "adjustment"), Some("split"));
    assert_eq!(value(&q, "feed"), Some("iex"));
}
