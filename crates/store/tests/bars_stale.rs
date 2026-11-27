//! R-418: a bar version older than the stored one is refused.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::market::{BarRow, UpsertStats};
use rust_decimal::Decimal;
use std::str::FromStr;

fn t(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, h, 0, 0).unwrap()
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
fn an_older_version_of_a_stored_bar_is_refused_and_nothing_is_stored() {
    // R-418
    let s = Store::open_in_memory().unwrap();
    s.upsert_bars(&[row("SPY", "100.5")], t(12)).unwrap();
    let err = s
        .upsert_bars(&[row("SPY", "100.9"), row("TLT", "90")], t(10))
        .unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)), "got {err:?}");
    assert!(read(&s, "TLT").is_empty());
    let spy = read(&s, "SPY");
    assert_eq!(spy.len(), 1);
    assert_eq!(spy[0].close.value(), Decimal::from_str("100.5").unwrap());
}

#[test]
fn an_older_fetch_of_an_equal_bar_is_unchanged() {
    // R-418
    let s = Store::open_in_memory().unwrap();
    s.upsert_bars(&[row("SPY", "100.5")], t(12)).unwrap();
    let stats = s.upsert_bars(&[row("SPY", "100.5")], t(10)).unwrap();
    assert_eq!(
        stats,
        UpsertStats {
            inserted: 0,
            unchanged: 1,
            restated: 0
        }
    );
}
