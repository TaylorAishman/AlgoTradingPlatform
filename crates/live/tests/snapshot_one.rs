//! R-300: the paper book is valued at the stored raw close of each completed session.

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

fn store_bars(fx: &LiveFixture) {
    fx.store
        .upsert_bars(
            &[spy_bar("2024-02-29", "140"), spy_bar("2024-03-01", "143")],
            fx.now(),
        )
        .expect("bars");
}

fn opening(fx: &LiveFixture) {
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
}

fn fill(fx: &LiveFixture) {
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
}

fn first_snapshot(fx: &LiveFixture) {
    fx.store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date: d(2024, 2, 29),
            cash: dec("100000"),
            positions_value: dec("0"),
            equity: dec("100000"),
            recorded_at: fx.now(),
        })
        .expect("snapshot");
}

// R-300
#[test]
fn a_session_after_the_last_snapshot_is_valued_at_its_raw_close() {
    let fx = LiveFixture::new(Stage::Paper);
    opening(&fx);
    first_snapshot(&fx);
    fill(&fx);
    store_bars(&fx);

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 1)).expect("snapshot");

    assert_eq!(n, 1);
    let series = fx.store.equity_series(Book::Paper).expect("series");
    assert_eq!(series.len(), 2);
    let s = &series[1];
    assert_eq!(s.date, d(2024, 3, 1));
    assert_eq!(s.cash, dec("98590"));
    assert_eq!(s.positions_value, dec("1430"));
    assert_eq!(s.equity, dec("100020"));
}

// R-300
#[test]
fn calling_again_stores_nothing_more() {
    let fx = LiveFixture::new(Stage::Paper);
    opening(&fx);
    first_snapshot(&fx);
    fill(&fx);
    store_bars(&fx);
    assert_eq!(snapshot_equity(&fx.ctx(), d(2024, 3, 1)).expect("first"), 1);

    assert_eq!(snapshot_equity(&fx.ctx(), d(2024, 3, 1)).expect("again"), 0);
    assert_eq!(
        fx.store.equity_series(Book::Paper).expect("series").len(),
        2
    );
}

// R-300
#[test]
fn with_no_snapshot_the_first_call_starts_at_the_opening_balance_date() {
    let fx = LiveFixture::new(Stage::Paper);
    opening(&fx);
    fill(&fx);
    store_bars(&fx);

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 1)).expect("snapshot");

    assert_eq!(n, 2);
    let series = fx.store.equity_series(Book::Paper).expect("series");
    let dates: Vec<_> = series.iter().map(|s| s.date).collect();
    assert_eq!(dates, [d(2024, 2, 29), d(2024, 3, 1)]);
    assert_eq!(series[0].cash, dec("100000"));
    assert_eq!(series[0].positions_value, dec("0"));
    assert_eq!(series[0].equity, dec("100000"));
    assert_eq!(series[1].equity, dec("100020"));
}

// R-300
#[test]
fn an_empty_ledger_stores_no_snapshot() {
    let fx = LiveFixture::new(Stage::Paper);
    store_bars(&fx);

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 1)).expect("snapshot");

    assert_eq!(n, 0);
    assert!(fx
        .store
        .equity_series(Book::Paper)
        .expect("series")
        .is_empty());
}
