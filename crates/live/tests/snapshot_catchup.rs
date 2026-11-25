//! R-301: snapshots catch up over every missed session and say that runs were missed.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar};
use pt_gate::owner::Stage;
use pt_live::snapshot::snapshot_equity;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, LedgerEntry};
use rust_decimal::Decimal;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("decimal")
}

fn spy_bar(date: &str, close: &str) -> BarRow {
    let p = Price::new(dec(close)).expect("price");
    BarRow {
        bar: Bar {
            symbol: sym("SPY"),
            date: date.parse().expect("date"),
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 1_000,
        },
        adjustment: Adjustment::Raw,
        feed: "sip".to_string(),
    }
}

/// Paper book holding 10 SPY, with the last snapshot dated 2024-03-01.
fn book_snapshotted_through_march_first(fx: &LiveFixture) {
    fx.store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: d(2024, 2, 29),
            kind: "opening".to_string(),
            symbol: None,
            qty: 0,
            price: None,
            cash_delta: dec("100000"),
            ref_id: None,
        })
        .expect("opening");
    fx.store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: d(2024, 3, 1),
            kind: "fill".to_string(),
            symbol: Some(sym("SPY")),
            qty: 10,
            price: Some(dec("141")),
            cash_delta: dec("-1410"),
            ref_id: None,
        })
        .expect("fill");
    fx.store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date: d(2024, 3, 1),
            cash: dec("98590"),
            positions_value: dec("1430"),
            equity: dec("100020"),
            recorded_at: fx.now(),
        })
        .expect("snapshot");
}

fn store_bars(fx: &LiveFixture, bars: &[(&str, &str)]) {
    let rows: Vec<BarRow> = bars.iter().map(|(dt, c)| spy_bar(dt, c)).collect();
    fx.store.upsert_bars(&rows, fx.now()).expect("bars");
}

fn missed_run_alerts(fx: &LiveFixture) -> Vec<(String, String, String, bool)> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == "missed_run")
        .collect()
}

// R-301
#[test]
fn every_missed_session_is_snapshotted_and_a_missed_run_warning_is_recorded() {
    let fx = LiveFixture::new(Stage::Paper);
    book_snapshotted_through_march_first(&fx);
    store_bars(
        &fx,
        &[
            ("2024-03-04", "144"),
            ("2024-03-05", "145"),
            ("2024-03-06", "146"),
        ],
    );

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 6)).expect("snapshot");

    assert_eq!(n, 3);
    let series = fx.store.equity_series(Book::Paper).expect("series");
    let dates: Vec<_> = series.iter().map(|s| s.date).collect();
    assert_eq!(
        dates,
        [d(2024, 3, 1), d(2024, 3, 4), d(2024, 3, 5), d(2024, 3, 6)]
    );
    assert_eq!(series[3].positions_value, dec("1460"));
    assert_eq!(series[3].equity, dec("100050"));
    let missed = missed_run_alerts(&fx);
    assert_eq!(missed.len(), 1);
    assert_eq!(missed[0].0, "warn");
}

// R-301
#[test]
fn a_single_new_session_is_not_a_missed_run() {
    let fx = LiveFixture::new(Stage::Paper);
    book_snapshotted_through_march_first(&fx);
    store_bars(&fx, &[("2024-03-04", "144")]);

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 4)).expect("snapshot");

    assert_eq!(n, 1);
    assert!(missed_run_alerts(&fx).is_empty());
}

// R-301
#[test]
fn a_session_lacking_a_bar_and_every_session_after_it_are_not_snapshotted() {
    let fx = LiveFixture::new(Stage::Paper);
    book_snapshotted_through_march_first(&fx);
    store_bars(&fx, &[("2024-03-04", "144"), ("2024-03-06", "146")]);

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 6)).expect("snapshot");

    assert_eq!(n, 1);
    let series = fx.store.equity_series(Book::Paper).expect("series");
    let dates: Vec<_> = series.iter().map(|s| s.date).collect();
    assert_eq!(dates, [d(2024, 3, 1), d(2024, 3, 4)]);
}
