mod common;

use common::{calendar_2024, d};
use pt_calendar::calendar::TradingCalendar;

// R-024
#[test]
fn month_ends_of_2024_exclude_the_last_session_of_the_calendar() {
    let expected = vec![
        d(2024, 1, 31),
        d(2024, 2, 29),
        d(2024, 3, 28),
        d(2024, 4, 30),
        d(2024, 5, 31),
        d(2024, 6, 28),
        d(2024, 7, 31),
        d(2024, 8, 30),
        d(2024, 9, 30),
        d(2024, 10, 31),
        d(2024, 11, 29),
    ];
    assert_eq!(
        calendar_2024().month_end_sessions(d(2024, 1, 1), d(2024, 12, 31)),
        expected
    );
}

// R-024
#[test]
fn last_session_of_december_is_absent_because_no_later_session_is_known() {
    let got = calendar_2024().month_end_sessions(d(2024, 12, 1), d(2024, 12, 31));
    assert!(got.is_empty());
}

// R-024
#[test]
fn month_end_is_recognised_when_the_later_session_is_in_the_next_year() {
    let cal = TradingCalendar::new(vec![
        d(2024, 12, 30),
        d(2024, 12, 31),
        d(2025, 1, 2),
        d(2025, 1, 3),
    ])
    .expect("valid calendar");
    assert_eq!(
        cal.month_end_sessions(d(2024, 12, 1), d(2025, 1, 31)),
        vec![d(2024, 12, 31)]
    );
}

// R-024
#[test]
fn range_bounds_are_inclusive() {
    let cal = calendar_2024();
    assert_eq!(
        cal.month_end_sessions(d(2024, 1, 31), d(2024, 2, 29)),
        vec![d(2024, 1, 31), d(2024, 2, 29)]
    );
}

// R-024
#[test]
fn range_without_a_month_end_is_empty() {
    let cal = calendar_2024();
    assert!(cal
        .month_end_sessions(d(2024, 2, 1), d(2024, 2, 28))
        .is_empty());
}

// R-024
#[test]
fn inverted_range_is_empty() {
    let cal = calendar_2024();
    assert!(cal
        .month_end_sessions(d(2024, 6, 30), d(2024, 1, 1))
        .is_empty());
}
