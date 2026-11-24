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
    let limiter = RateLimiter::new(200, mono, sleeper.clone());
    let endpoints = Endpoints::with_overrides(Some(trading), Some(data)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    )
}

fn page(days: &[u32], token: Option<&str>) -> String {
    let bars: Vec<String> = days
        .iter()
        .map(|d| {
            format!(r#"{{"t":"2024-01-{d:02}T05:00:00Z","o":1.5,"h":2.5,"l":1.25,"c":2.0,"v":10}}"#)
        })
        .collect();
    let token = match token {
        Some(t) => format!(r#""{t}""#),
        None => "null".to_string(),
    };
    format!(
        r#"{{"bars":{{"SPY":[{}]}},"next_page_token":{token}}}"#,
        bars.join(",")
    )
}

fn has_page_token(query: &str) -> bool {
    query.split('&').any(|p| p.starts_with("page_token="))
}

// R-090: three pages of 2, 2 and 1 bars come back as 5 bars in served order, after three
// requests, the second with page_token=a and the third with page_token=b.
#[test]
fn three_pages_are_followed_to_the_end_in_order() {
    let l = Listener::serve(vec![
        (200, page(&[2, 3], Some("a"))),
        (200, page(&[4, 5], Some("b"))),
        (200, page(&[8], None)),
    ]);
    let http = client(DEAD_TRADING, &l.url);
    let symbols = vec!["SPY".to_string()];
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    let result = daily_bars(&http, &symbols, start, end, "all", "sip");
    // Check the result before finish(): a client that stops early must fail, not hang.
    let bars = result.expect("pages are followed");
    let dates: Vec<NaiveDate> = bars.iter().map(|b| b.date).collect();
    let expected: Vec<NaiveDate> = [2, 3, 4, 5, 8]
        .iter()
        .map(|d| NaiveDate::from_ymd_opt(2024, 1, *d).unwrap())
        .collect();
    assert_eq!(dates, expected);
    let seen = l.finish();

    assert_eq!(seen.len(), 3);
    assert!(!has_page_token(seen[0].query()), "{}", seen[0].url);
    assert!(
        seen[1].query().split('&').any(|p| p == "page_token=a"),
        "{}",
        seen[1].url
    );
    assert!(
        seen[2].query().split('&').any(|p| p == "page_token=b"),
        "{}",
        seen[2].url
    );
    for s in &seen {
        assert_eq!(s.path(), "/v2/stocks/bars");
    }
}

// R-090: a single page with a null token makes exactly one request.
#[test]
fn a_null_token_on_the_first_page_stops_after_one_request() {
    let l = Listener::serve(vec![(200, page(&[2, 3], None))]);
    let http = client(DEAD_TRADING, &l.url);
    let symbols = vec!["SPY".to_string()];
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    let result = daily_bars(&http, &symbols, start, end, "all", "sip");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert!(!has_page_token(seen[0].query()));
    assert_eq!(result.expect("one page").len(), 2);
}
