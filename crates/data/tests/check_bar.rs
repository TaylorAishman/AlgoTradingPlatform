//! R-124: a bar must be internally consistent.

use chrono::NaiveDate;
use pt_alpaca::data::WireBar;
use pt_data::quality::check_bar;
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn bar(open: &str, high: &str, low: &str, close: &str) -> WireBar {
    WireBar {
        symbol: "SPY".to_string(),
        date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
        open: d(open),
        high: d(high),
        low: d(low),
        close: d(close),
        volume: 1000,
    }
}

#[test]
fn consistent_bar_is_accepted() {
    // R-124
    assert_eq!(check_bar(&bar("10", "11", "9", "10.5")), Ok(()));
}

#[test]
fn high_below_close_is_rejected() {
    // R-124
    assert!(check_bar(&bar("10", "10.4", "9", "10.5")).is_err());
}

#[test]
fn low_above_open_is_rejected() {
    // R-124
    assert!(check_bar(&bar("10", "11", "10.1", "10.5")).is_err());
}

#[test]
fn zero_low_is_rejected() {
    // R-124
    assert!(check_bar(&bar("10", "11", "0", "10.5")).is_err());
}

#[test]
fn bad_bar_error_names_symbol_and_date_with_a_message() {
    // R-444
    match check_bar(&bar("10", "10.4", "9", "10.5")) {
        Err(pt_data::error::DataError::BadBar {
            symbol,
            date,
            message,
        }) => {
            assert_eq!(symbol, "SPY");
            assert_eq!(date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
            assert!(!message.is_empty());
        }
        other => panic!("expected BadBar, got {other:?}"),
    }
}
