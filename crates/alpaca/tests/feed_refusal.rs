mod common;

use std::sync::Arc;

use chrono::NaiveDate;
use pt_alpaca::auth::Credentials;
use pt_alpaca::data::daily_bars;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};

use common::{Fake, Listener};

/// Nothing listens here, so a request sent to the trading host fails at once.
const DEAD_TRADING: &str = "http://127.0.0.1:1";

const REFUSAL: &str = r#"{"message":"subscription does not permit querying recent SIP data"}"#;

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

fn request(http: &HttpClient, feed: &str) -> Result<Vec<pt_alpaca::data::WireBar>, AlpacaError> {
    let symbols = vec!["SPY".to_string()];
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    daily_bars(http, &symbols, start, end, "all", feed)
}

// R-092: a 403 on the bars request for the sip feed is FeedNotPermitted with the feed and
// the server message, after one request.
#[test]
fn a_403_for_sip_is_reported_as_feed_not_permitted() {
    let l = Listener::serve(vec![(403, REFUSAL.to_string())]);
    let http = client(DEAD_TRADING, &l.url);
    let result = request(&http, "sip");
    // Check the result before finish(): a client that retries must fail, not hang.
    let err = result.expect_err("a refused feed is an error");
    match err {
        AlpacaError::FeedNotPermitted { feed, message } => {
            assert_eq!(feed, "sip");
            assert!(
                message.contains("subscription does not permit querying recent SIP data"),
                "{message}"
            );
        }
        other => panic!("expected FeedNotPermitted, got {other:?}"),
    }
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path(), "/v2/stocks/bars");
}

// R-092: the reported feed is the one requested, not a fixed name.
#[test]
fn the_reported_feed_is_the_one_requested() {
    let l = Listener::serve(vec![(403, REFUSAL.to_string())]);
    let http = client(DEAD_TRADING, &l.url);
    let result = request(&http, "iex");
    let err = result.expect_err("a refused feed is an error");
    assert!(
        matches!(&err, AlpacaError::FeedNotPermitted { feed, .. } if feed == "iex"),
        "{err:?}"
    );
    assert_eq!(l.finish().len(), 1);
}
