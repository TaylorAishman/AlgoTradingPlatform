//! R-123: a restated adjusted history is kept as a new version.

mod ingest_common;

use ingest_common::*;
use pt_alpaca::http::loopback_client;
use pt_core::money::Price;
use pt_core::types::{Adjustment, Symbol};
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;

/// Ingests the original history at t1, then a SPY adjusted history with every close
/// multiplied by 0.99 at t2. Returns the store and the second report's adjusted stats.
fn ingest_then_restate() -> (Store, pt_store::market::UpsertStats) {
    let first = MockAlpaca::start();
    seed(&first, &SYMBOLS, &SESSIONS);
    let http = loopback_client(&first.url()).expect("loopback");
    let store = store_with_calendar();
    ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect("first ingest");

    // The mock appends seeds, so the restated history gets a fresh mock.
    let second = MockAlpaca::start();
    seed(&second, &["TLT", "BIL"], &SESSIONS);
    second.seed_bars(
        "raw",
        "sip",
        SESSIONS
            .iter()
            .map(|d| {
                (
                    "SPY".to_string(),
                    d.to_string(),
                    100.0,
                    101.0,
                    99.0,
                    100.5,
                    1000,
                )
            })
            .collect(),
    );
    second.seed_bars(
        "all",
        "sip",
        SESSIONS
            .iter()
            .map(|d| {
                (
                    "SPY".to_string(),
                    d.to_string(),
                    100.0,
                    101.0,
                    99.0,
                    99.495,
                    1000,
                )
            })
            .collect(),
    );
    let http = loopback_client(&second.url()).expect("loopback");
    let report = ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-17T22:00:00Z"),
    )
    .expect("second ingest");
    (store, report.adjusted)
}

fn spy_closes(store: &Store, asof: Option<&str>) -> Vec<Price> {
    store
        .bars_asof(
            &Symbol::new("SPY").unwrap(),
            Adjustment::All,
            "sip",
            date("2024-01-01"),
            date("2024-01-16"),
            asof.map(instant),
        )
        .expect("bars_asof")
        .iter()
        .map(|b| b.close)
        .collect()
}

// R-123: ten restated SPY adjusted bars are counted as restated.
#[test]
fn restated_adjusted_history_is_counted() {
    let (_store, adjusted) = ingest_then_restate();
    assert_eq!(adjusted.restated, 10);
}

// R-123: as of t1 the original closes remain; as of now the new ones are returned.
#[test]
fn bars_asof_t1_keeps_original_closes_and_latest_has_restated() {
    let (store, _) = ingest_then_restate();
    let old = spy_closes(&store, Some("2024-01-16T22:00:00Z"));
    let new = spy_closes(&store, None);
    let price = |s: &str| Price::new(s.parse().expect("decimal")).expect("price");
    assert_eq!(old, vec![price("100.5"); 10]);
    assert_eq!(new, vec![price("99.495"); 10]);
}
