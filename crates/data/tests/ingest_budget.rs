//! R-127, R-128: ingest stays inside the request budget and surfaces a feed refusal.

mod ingest_common;

use chrono::{Datelike, Duration, Weekday};
use ingest_common::*;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::loopback_client;
use pt_core::types::{Adjustment, Symbol};
use pt_data::error::DataError;
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;

/// The first 250 weekdays from 2024-01-01, as `YYYY-MM-DD` strings.
fn two_hundred_fifty_sessions() -> Vec<String> {
    let mut day = date("2024-01-01");
    let mut out = Vec::new();
    while out.len() < 250 {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(day.to_string());
        }
        day += Duration::days(1);
    }
    out
}

// R-127: 3 symbols x 250 sessions are fetched with at most 4 bars requests in total.
#[test]
fn a_full_backfill_makes_at_most_four_bars_requests() {
    let sessions = two_hundred_fifty_sessions();
    let refs: Vec<&str> = sessions.iter().map(String::as_str).collect();
    let mock = MockAlpaca::start();
    seed(&mock, &SYMBOLS, &refs);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = Store::open_in_memory().expect("store");

    let report = ingest(
        &store,
        &http,
        &config(),
        date(refs[249]),
        instant("2025-01-01T22:00:00Z"),
    )
    .expect("ingest succeeds");

    assert_eq!(report.raw.inserted, 750);
    assert_eq!(report.adjusted.inserted, 750);
    let bars_requests = mock
        .requests()
        .iter()
        .filter(|r| r.path == "/v2/stocks/bars")
        .count();
    assert!(
        (1..=4).contains(&bars_requests),
        "expected at most 4 bars requests, got {bars_requests}"
    );
}

// R-128: a denied feed returns FeedNotPermitted unchanged and stores no bars.
#[test]
fn a_denied_feed_surfaces_feed_not_permitted_and_stores_nothing() {
    let mock = MockAlpaca::start();
    seed(&mock, &SYMBOLS, &SESSIONS);
    mock.deny_feed("sip");
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_calendar();

    let err = ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect_err("ingest must fail on a denied feed");

    assert!(
        matches!(
            err,
            DataError::Alpaca(AlpacaError::FeedNotPermitted { ref feed, .. }) if feed == "sip"
        ),
        "expected FeedNotPermitted, got {err:?}"
    );
    for name in SYMBOLS {
        let symbol = Symbol::new(name).expect("symbol");
        for adjustment in [Adjustment::Raw, Adjustment::All] {
            let bars = store
                .bars_asof(
                    &symbol,
                    adjustment,
                    "sip",
                    date("2024-01-01"),
                    date("2024-01-16"),
                    None,
                )
                .expect("bars");
            assert!(bars.is_empty(), "{name} {adjustment:?} should hold no bars");
        }
    }
}
