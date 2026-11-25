//! R-131: the calendar must be importable from CSV.

use chrono::{Datelike, NaiveDate, Weekday};
use pt_data::csv_import::import_calendar_csv;
use pt_data::error::DataError;
use pt_store::db::Store;

/// 252 fake session dates: the first 252 weekdays of 2024.
fn fake_sessions() -> Vec<NaiveDate> {
    let mut d = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let mut out = Vec::new();
    while out.len() < 252 {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(d);
        }
        d = d.succ_opt().unwrap();
    }
    out
}

fn csv_of(dates: &[NaiveDate]) -> String {
    let mut text = String::from("date\n");
    for d in dates {
        text.push_str(&format!("{d}\n"));
    }
    text
}

#[test]
fn importing_252_dates_returns_252_and_store_calendar_holds_them() {
    // R-131
    let store = Store::open_in_memory().unwrap();
    let dates = fake_sessions();
    let n = import_calendar_csv(&store, &csv_of(&dates)).unwrap();
    assert_eq!(n, 252);
    let cal = store.calendar().unwrap();
    let first = *dates.first().unwrap();
    let last = *dates.last().unwrap();
    assert_eq!(cal.sessions_between(first, last), dates.as_slice());
}

#[test]
fn invalid_month_row_returns_csv_error_with_line_and_stores_nothing() {
    // R-131
    let store = Store::open_in_memory().unwrap();
    let text = "date\n2024-01-02\n2024-01-03\n2024-13-01\n";
    let err = import_calendar_csv(&store, text).unwrap_err();
    assert!(matches!(err, DataError::Csv { line: 4, .. }), "got {err:?}");
    assert!(
        store.calendar().is_err()
            || store
                .calendar()
                .unwrap()
                .sessions_between(
                    NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()
                )
                .is_empty()
    );
}
