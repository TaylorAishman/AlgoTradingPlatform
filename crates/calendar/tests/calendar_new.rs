mod common;

use common::{calendar_2024, d, sessions_2024};
use pt_calendar::calendar::TradingCalendar;
use pt_calendar::error::CalendarError;

// R-020
#[test]
fn empty_list_is_rejected_as_empty() {
    assert_eq!(
        TradingCalendar::new(vec![]).err(),
        Some(CalendarError::Empty)
    );
}

// R-020
#[test]
fn descending_pair_is_rejected_as_not_sorted() {
    let r = TradingCalendar::new(vec![d(2024, 1, 3), d(2024, 1, 2)]);
    assert_eq!(r.err(), Some(CalendarError::NotSorted));
}

// R-020
#[test]
fn repeated_date_is_rejected_as_duplicate() {
    let r = TradingCalendar::new(vec![d(2024, 1, 2), d(2024, 1, 2)]);
    assert_eq!(r.err(), Some(CalendarError::Duplicate(d(2024, 1, 2))));
}

// R-020
#[test]
fn single_session_is_accepted() {
    let cal = TradingCalendar::new(vec![d(2024, 1, 2)]).expect("valid");
    assert!(cal.is_session(d(2024, 1, 2)));
}

// R-020
#[test]
fn fixture_has_252_sessions_and_builds() {
    assert_eq!(sessions_2024().len(), 252);
    let _ = calendar_2024();
}

// R-020
#[test]
fn is_session_is_true_only_for_listed_dates() {
    let cal = calendar_2024();
    assert!(cal.is_session(d(2024, 1, 2)));
    assert!(cal.is_session(d(2024, 12, 31)));
    assert!(!cal.is_session(d(2024, 3, 29))); // Good Friday
    assert!(!cal.is_session(d(2024, 3, 30))); // Saturday
    assert!(!cal.is_session(d(2023, 12, 29))); // before the first session
    assert!(!cal.is_session(d(2025, 1, 2))); // after the last session
}
