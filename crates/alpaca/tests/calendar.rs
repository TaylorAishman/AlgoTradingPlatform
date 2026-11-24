mod common;

use std::sync::Arc;

use chrono::{DateTime, NaiveDate};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::reference::{calendar, clock};

use common::{Fake, Listener};

const CALENDAR_BODY: &str = include_str!("fixtures/calendar.json");
const CLOCK_BODY: &str = r#"{"timestamp":"2025-06-24T14:15:22-04:00","is_open":true,"next_open":"2025-06-25T09:30:00-04:00","next_close":"2025-06-24T16:00:00-04:00"}"#;

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

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("date")
}

// R-095: the clock keeps its flag and the three instants with their offsets.
#[test]
fn clock_preserves_instants_and_offsets() {
    let l = Listener::serve(vec![(200, CLOCK_BODY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let c = clock(&http).expect("clock");
    let seen = l.finish();
    assert_eq!(seen[0].path(), "/v2/clock");
    assert!(c.is_open);
    assert_eq!(
        c.timestamp,
        DateTime::parse_from_rfc3339("2025-06-24T14:15:22-04:00").unwrap()
    );
    assert_eq!(c.timestamp.offset().local_minus_utc(), -4 * 3600);
    assert_eq!(c.next_open.to_rfc3339(), "2025-06-25T09:30:00-04:00");
    assert_eq!(c.next_close.to_rfc3339(), "2025-06-24T16:00:00-04:00");
}

// R-095: the calendar sends start and end and returns the session dates.
#[test]
fn calendar_sends_range_and_parses_dates() {
    let l = Listener::serve(vec![(200, CALENDAR_BODY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let days = calendar(&http, date(2024, 1, 2), date(2024, 1, 4)).expect("calendar");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/calendar");
    let q: Vec<&str> = seen[0].query().split('&').collect();
    assert!(q.contains(&"start=2024-01-02"), "query: {q:?}");
    assert!(q.contains(&"end=2024-01-04"), "query: {q:?}");
    assert_eq!(days, vec![date(2024, 1, 2)]);
}

// R-095: an empty calendar range is an empty list.
#[test]
fn empty_calendar_is_an_empty_list() {
    let l = Listener::serve(vec![(200, "[]".to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let days = calendar(&http, date(2024, 1, 6), date(2024, 1, 7)).expect("calendar");
    l.finish();
    assert!(days.is_empty());
}

// R-095: a date that does not parse is a parse error.
#[test]
fn bad_calendar_date_is_a_parse_error() {
    let body = r#"[{"date":"not-a-date","open":"09:30","close":"16:00"}]"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let r = calendar(&http, date(2024, 1, 2), date(2024, 1, 4));
    l.finish();
    assert!(matches!(r, Err(AlpacaError::Parse(_))), "got {r:?}");
}
