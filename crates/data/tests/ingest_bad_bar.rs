//! R-125: ingest stores nothing from a response that contains a bad bar.

mod ingest_common;

use ingest_common::*;
use pt_alpaca::http::loopback_client;
use pt_core::types::{Adjustment, Symbol};
use pt_data::error::DataError;
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;

/// Seeds every symbol and session, except that SPY on 2024-01-09 has a high
/// below its close in the `bad_series` series.
fn seeded_with_bad_spy_bar(bad_series: &str) -> MockAlpaca {
    let mock = MockAlpaca::start();
    for adjustment in ["raw", "all"] {
        let bars = SYMBOLS
            .iter()
            .flat_map(|s| SESSIONS.iter().map(move |d| (*s, *d)))
            .map(|(s, d)| {
                let bad = adjustment == bad_series && s == "SPY" && d == "2024-01-09";
                let (high, low) = if bad { (99.0, 98.0) } else { (101.0, 99.0) };
                (s.to_string(), d.to_string(), 100.0, high, low, 100.5, 1000)
            })
            .collect();
        mock.seed_bars(adjustment, "sip", bars);
    }
    mock
}

fn assert_store_empty(store: &Store) {
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

fn run_and_expect_bad_bar(bad_series: &str) {
    let mock = seeded_with_bad_spy_bar(bad_series);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_calendar();

    let err = ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect_err("ingest must refuse a bad bar");

    match err {
        DataError::BadBar {
            symbol, date: d, ..
        } => {
            assert_eq!(symbol, "SPY");
            assert_eq!(d, date("2024-01-09"));
        }
        other => panic!("expected BadBar, got {other:?}"),
    }
    assert_store_empty(&store);
}

// R-125: SPY high below close in the raw series -> BadBar naming SPY and the date; no bars stored.
#[test]
fn a_bad_raw_bar_is_refused_and_nothing_is_stored() {
    run_and_expect_bad_bar("raw");
}

// R-125: a bad bar in the adjusted series also stores nothing, not even the good raw series.
#[test]
fn a_bad_adjusted_bar_is_refused_and_nothing_is_stored() {
    run_and_expect_bad_bar("all");
}
