//! R-445: the raw refetch starts five sessions before the earliest latest stored raw bar.

mod ingest_common;

use ingest_common::*;
use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;

fn config_from_first_session() -> Config {
    parse(&format!("{UNIVERSE}\n[data]\nstart = \"2024-01-02\"\n")).expect("config parses")
}

/// Ingests through 2024-01-16 with the given seeding, then ingests through
/// 2024-01-17 and returns the query of the second raw request.
fn second_raw_query(seed_all: impl FnOnce(&MockAlpaca)) -> String {
    let mock = MockAlpaca::start();
    seed_all(&mock);
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

    let before = mock.requests().len();
    ingest(
        &store,
        &http,
        &cfg,
        date("2024-01-17"),
        instant("2024-01-17T22:00:00Z"),
    )
    .expect("second ingest");
    let requests = mock.requests();
    requests[before..]
        .iter()
        .filter(|r| r.path == "/v2/stocks/bars")
        .find(|r| r.query.contains("adjustment=raw"))
        .expect("a raw request")
        .query
        .clone()
}

// R-445: SPY stored through 01-16, TLT and BIL through 01-12 (15th a holiday):
// the anchor is five sessions before 01-12, which is 01-05.
#[test]
fn raw_refetch_starts_five_sessions_before_the_earliest_latest_bar() {
    let query = second_raw_query(|mock| {
        seed(mock, &["SPY"], &SESSIONS);
        seed(mock, &["TLT", "BIL"], &SESSIONS[..9]);
    });
    assert!(query.contains("start=2024-01-05"), "raw query: {query}");
}

// R-445: with no raw bar stored for BIL the raw request starts at data.start.
#[test]
fn raw_refetch_starts_at_data_start_when_a_symbol_has_no_raw_bar() {
    let query = second_raw_query(|mock| {
        seed(mock, &["SPY", "TLT"], &SESSIONS);
    });
    assert!(query.contains("start=2024-01-02"), "raw query: {query}");
}
