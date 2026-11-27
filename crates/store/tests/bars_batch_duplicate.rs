//! R-417: two different bars for one key in one batch are refused as a whole.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::market::{BarRow, UpsertStats};
use rust_decimal::Decimal;
use std::str::FromStr;

fn t() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 12, 0, 0).unwrap()
}

fn price(s: &str) -> Price {
    Price::new(Decimal::from_str(s).unwrap()).unwrap()
}

fn row(symbol: &str, close: &str) -> BarRow {
    BarRow {
        bar: Bar {
            symbol: Symbol::new(symbol).unwrap(),
            date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            open: price("99.5"),
            high: price("101.25"),
            low: price("98.75"),
            close: price(close),
            volume: 1_000,
        },
        adjustment: Adjustment::Raw,
        feed: "sip".to_string(),
    }
}

fn read(s: &Store, symbol: &str) -> Vec<Bar> {
    let d = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    s.bars_asof(
        &Symbol::new(symbol).unwrap(),
        Adjustment::Raw,
        "sip",
        d,
        d,
        None,
    )
    .unwrap()
}

#[test]
fn conflicting_bars_for_one_key_refuse_the_whole_batch() {
    // R-417
    let s = Store::open_in_memory().unwrap();
    let batch = vec![row("SPY", "100.5"), row("TLT", "90"), row("SPY", "100.6")];
    let err = s.upsert_bars(&batch, t()).unwrap_err();
    match err {
        StoreError::Duplicate(m) => {
            assert!(m.contains("SPY"), "message: {m}");
            assert!(m.contains("2024-01-02"), "message: {m}");
        }
        other => panic!("expected Duplicate, got {other:?}"),
    }
    assert!(read(&s, "SPY").is_empty());
    assert!(read(&s, "TLT").is_empty());
}

#[test]
fn the_same_bar_twice_in_a_batch_is_inserted_once() {
    // R-417
    let s = Store::open_in_memory().unwrap();
    let batch = vec![row("SPY", "100.5"), row("SPY", "100.5")];
    let stats = s.upsert_bars(&batch, t()).unwrap();
    assert_eq!(
        stats,
        UpsertStats {
            inserted: 1,
            unchanged: 1,
            restated: 0
        }
    );
    assert_eq!(read(&s, "SPY").len(), 1);
}
