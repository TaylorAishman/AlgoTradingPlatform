//! R-129: bars must be importable from CSV.

use chrono::{NaiveDate, TimeZone, Utc};
use pt_core::types::{Adjustment, Symbol};
use pt_data::csv_import::import_bars_csv;
use pt_store::db::Store;
use rust_decimal::Decimal;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

fn spy() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn fetched() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 12, 0, 0).unwrap()
}

#[test]
fn imports_two_rows_and_reads_them_back_with_exact_decimals() {
    // R-129
    let store = Store::open_in_memory().unwrap();
    let text = "symbol,date,open,high,low,close,volume\n\
                SPY,2024-01-02,472.16,473.67,470.49,472.65,123456\n\
                SPY,2024-01-03,470.43,471.19,468.17,468.79,234567\n";
    let n = import_bars_csv(&store, text, Adjustment::All, "stooq", fetched()).unwrap();
    assert_eq!(n, 2);

    let bars = store
        .bars_asof(&spy(), Adjustment::All, "csv:stooq", d(1), d(31), None)
        .unwrap();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].date, d(2));
    assert_eq!(bars[0].open.value(), dec("472.16"));
    assert_eq!(bars[0].high.value(), dec("473.67"));
    assert_eq!(bars[0].low.value(), dec("470.49"));
    assert_eq!(bars[0].close.value(), dec("472.65"));
    assert_eq!(bars[0].volume, 123_456);
    assert_eq!(bars[1].date, d(3));
    assert_eq!(bars[1].close.value(), dec("468.79"));
    assert_eq!(bars[1].volume, 234_567);
}

#[test]
fn columns_may_appear_in_any_order() {
    // R-129
    let store = Store::open_in_memory().unwrap();
    let text = "volume,close,date,low,symbol,high,open\n\
                123456,472.65,2024-01-02,470.49,SPY,473.67,472.16\n\
                234567,468.79,2024-01-03,468.17,SPY,471.19,470.43\n";
    let n = import_bars_csv(&store, text, Adjustment::All, "stooq", fetched()).unwrap();
    assert_eq!(n, 2);

    let bars = store
        .bars_asof(&spy(), Adjustment::All, "csv:stooq", d(1), d(31), None)
        .unwrap();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].open.value(), dec("472.16"));
    assert_eq!(bars[0].high.value(), dec("473.67"));
    assert_eq!(bars[0].low.value(), dec("470.49"));
    assert_eq!(bars[0].close.value(), dec("472.65"));
    assert_eq!(bars[0].volume, 123_456);
}

#[test]
fn imported_bars_are_stored_only_under_the_csv_feed_and_given_adjustment() {
    // R-129
    let store = Store::open_in_memory().unwrap();
    let text = "symbol,date,open,high,low,close,volume\n\
                SPY,2024-01-02,472.16,473.67,470.49,472.65,123456\n";
    import_bars_csv(&store, text, Adjustment::All, "stooq", fetched()).unwrap();

    let other_feed = store
        .bars_asof(&spy(), Adjustment::All, "sip", d(1), d(31), None)
        .unwrap();
    assert!(other_feed.is_empty());
    let raw = store
        .bars_asof(&spy(), Adjustment::Raw, "csv:stooq", d(1), d(31), None)
        .unwrap();
    assert!(raw.is_empty());
}
