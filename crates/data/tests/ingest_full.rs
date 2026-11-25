//! R-120: ingest stores raw and adjusted daily bars for every universe symbol and the risk-free symbol.

mod ingest_common;

use ingest_common::*;
use pt_alpaca::http::loopback_client;
use pt_core::types::{Adjustment, Symbol};
use pt_data::ingest::ingest;
use pt_mock_alpaca::server::MockAlpaca;

// R-120: 3 symbols x 10 sessions give 30 raw and 30 adjusted inserts, and the data is complete.
#[test]
fn ingest_reports_thirty_raw_and_thirty_adjusted_bars_complete() {
    let mock = MockAlpaca::start();
    seed(&mock, &SYMBOLS, &SESSIONS);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_calendar();

    let report = ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect("ingest succeeds");

    assert_eq!(report.raw.inserted, 30);
    assert_eq!(report.adjusted.inserted, 30);
    assert!(report.complete);
}

// R-120: afterwards bars_asof returns 10 bars per symbol and adjustment under feed sip.
#[test]
fn ingest_makes_ten_bars_readable_per_symbol_and_adjustment() {
    let mock = MockAlpaca::start();
    seed(&mock, &SYMBOLS, &SESSIONS);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_calendar();
    ingest(
        &store,
        &http,
        &config(),
        date("2024-01-16"),
        instant("2024-01-16T22:00:00Z"),
    )
    .expect("ingest succeeds");

    for symbol in SYMBOLS {
        for adjustment in [Adjustment::Raw, Adjustment::All] {
            let bars = store
                .bars_asof(
                    &Symbol::new(symbol).unwrap(),
                    adjustment,
                    "sip",
                    date("2024-01-01"),
                    date("2024-01-16"),
                    None,
                )
                .expect("bars_asof");
            assert_eq!(bars.len(), 10, "{symbol} {adjustment:?}");
        }
    }
}
