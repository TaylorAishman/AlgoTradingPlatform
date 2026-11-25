//! R-122: a repeated ingest refetches only recent raw bars but the whole adjusted history.

mod ingest_common;

use ingest_common::*;
use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;

/// The shared universe with the data start pinned to the first seeded session.
fn config_from_first_session() -> Config {
    parse(&format!("{UNIVERSE}\n[data]\nstart = \"2024-01-02\"\n")).expect("config parses")
}

// R-122: second ingest asks raw from five sessions back and adjusted from the start,
// and stores only the new session.
#[test]
fn second_ingest_refetches_recent_raw_and_all_adjusted() {
    let mock = MockAlpaca::start();
    seed(&mock, &SYMBOLS, &SESSIONS);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_calendar();
    let cfg = config_from_first_session();
    ingest(
        &store,
        &http,
        &cfg,
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect("first ingest");

    seed(&mock, &SYMBOLS, &["2024-01-17"]);
    let before = mock.requests().len();
    let report = ingest(
        &store,
        &http,
        &cfg,
        date("2024-01-17"),
        instant("2024-01-17T22:00:00Z"),
    )
    .expect("second ingest");

    let second: Vec<_> = mock.requests()[before..]
        .iter()
        .filter(|r| r.path == "/v2/stocks/bars")
        .cloned()
        .collect();
    let raw_request = second
        .iter()
        .find(|r| r.query.contains("adjustment=raw"))
        .expect("a raw request");
    let adjusted_request = second
        .iter()
        .find(|r| r.query.contains("adjustment=all"))
        .expect("an adjusted request");
    assert!(
        raw_request.query.contains("start=2024-01-08"),
        "raw query: {}",
        raw_request.query
    );
    assert!(
        adjusted_request.query.contains("start=2024-01-02"),
        "adjusted query: {}",
        adjusted_request.query
    );

    assert_eq!(report.raw.inserted, 3);
    assert_eq!(report.raw.unchanged, 18);
    assert_eq!(report.adjusted.inserted, 3);
    assert_eq!(report.adjusted.unchanged, 30);
    assert!(report.complete);
}
