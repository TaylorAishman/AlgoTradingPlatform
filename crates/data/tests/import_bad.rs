//! R-130: a malformed CSV must import nothing.

use chrono::{NaiveDate, TimeZone, Utc};
use pt_core::types::{Adjustment, Symbol};
use pt_data::csv_import::import_bars_csv;
use pt_data::error::DataError;
use pt_store::db::Store;

fn fetched() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 12, 0, 0).unwrap()
}

fn stored_count(store: &Store) -> usize {
    store
        .bars_asof(
            &Symbol::new("SPY").unwrap(),
            Adjustment::All,
            "csv:stooq",
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
            None,
        )
        .unwrap()
        .len()
}

#[test]
fn bad_close_in_third_data_row_names_line_4_and_stores_nothing() {
    // R-130
    let store = Store::open_in_memory().unwrap();
    let text = "symbol,date,open,high,low,close,volume\n\
                SPY,2024-01-02,472.16,473.67,470.49,472.65,123456\n\
                SPY,2024-01-03,470.43,471.19,468.17,468.79,234567\n\
                SPY,2024-01-04,468.00,469.00,467.00,abc,345678\n";
    let err = import_bars_csv(&store, text, Adjustment::All, "stooq", fetched()).unwrap_err();
    assert!(matches!(err, DataError::Csv { line: 4, .. }), "got {err:?}");
    assert_eq!(stored_count(&store), 0);
}

#[test]
fn missing_close_column_names_line_1_and_stores_nothing() {
    // R-130
    let store = Store::open_in_memory().unwrap();
    let text = "symbol,date,open,high,low,volume\n\
                SPY,2024-01-02,472.16,473.67,470.49,123456\n";
    let err = import_bars_csv(&store, text, Adjustment::All, "stooq", fetched()).unwrap_err();
    assert!(matches!(err, DataError::Csv { line: 1, .. }), "got {err:?}");
    assert_eq!(stored_count(&store), 0);
}
