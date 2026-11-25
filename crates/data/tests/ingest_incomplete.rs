//! R-121: ingest reports incomplete data instead of guessing.

mod ingest_common;

use ingest_common::*;
use pt_alpaca::http::loopback_client;
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;

fn run(mock: &MockAlpaca) -> pt_data::ingest::IngestReport {
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_calendar();
    ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect("ingest succeeds")
}

// R-121: no TLT bar on 2024-01-16 -> ingest succeeds with complete == false.
#[test]
fn ingest_is_incomplete_when_a_symbol_lacks_the_through_bar() {
    let mock = MockAlpaca::start();
    seed(&mock, &["SPY", "BIL"], &SESSIONS);
    seed(&mock, &["TLT"], &SESSIONS[..9]);

    let report = run(&mock);

    assert!(!report.complete);
    // What exists is still stored: 29 bars per series.
    assert_eq!(report.raw.inserted, 29);
    assert_eq!(report.adjusted.inserted, 29);
}

// R-121: a gap in only the adjusted series also makes the data incomplete.
#[test]
fn ingest_is_incomplete_when_only_the_adjusted_series_lacks_a_bar() {
    let mock = MockAlpaca::start();
    let bar = |s: &str, d: &str| {
        (
            s.to_string(),
            d.to_string(),
            100.0,
            101.0,
            99.0,
            100.5,
            1000,
        )
    };
    let all = |skip: Option<(&str, &str)>| {
        SYMBOLS
            .iter()
            .flat_map(|s| SESSIONS.iter().map(move |d| (*s, *d)))
            .filter(|p| Some(*p) != skip)
            .map(|(s, d)| bar(s, d))
            .collect::<Vec<_>>()
    };
    mock.seed_bars("raw", "sip", all(None));
    mock.seed_bars("all", "sip", all(Some(("TLT", "2024-01-16"))));

    let report = run(&mock);

    assert!(!report.complete);
}
