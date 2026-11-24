mod common;

use common::{calendar_2024, d, sessions_2024};

// R-022
#[test]
fn full_year_range_has_252_sessions() {
    let cal = calendar_2024();
    let got = cal.sessions_between(d(2024, 1, 1), d(2024, 12, 31));
    assert_eq!(got.len(), 252);
    assert_eq!(got, sessions_2024().as_slice());
}

// R-022
#[test]
fn range_is_inclusive_at_both_ends() {
    let cal = calendar_2024();
    let got = cal.sessions_between(d(2024, 1, 2), d(2024, 1, 5));
    assert_eq!(
        got,
        &[d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)]
    );
}

// R-022
#[test]
fn non_session_endpoints_are_handled() {
    let cal = calendar_2024();
    // 2024-03-29 (Good Friday) to 2024-03-31 (Sunday) contains no session.
    assert!(cal
        .sessions_between(d(2024, 3, 29), d(2024, 3, 31))
        .is_empty());
    // Saturday to Monday picks up only Monday.
    assert_eq!(
        cal.sessions_between(d(2024, 3, 30), d(2024, 4, 1)),
        &[d(2024, 4, 1)]
    );
}

// R-022
#[test]
fn single_day_range_on_a_session_has_one_entry() {
    let cal = calendar_2024();
    assert_eq!(
        cal.sessions_between(d(2024, 3, 28), d(2024, 3, 28)),
        &[d(2024, 3, 28)]
    );
}

// R-022
#[test]
fn from_after_to_is_empty() {
    let cal = calendar_2024();
    assert!(cal
        .sessions_between(d(2024, 1, 5), d(2024, 1, 2))
        .is_empty());
}

// R-022
#[test]
fn range_outside_the_calendar_is_empty() {
    let cal = calendar_2024();
    assert!(cal
        .sessions_between(d(2025, 1, 1), d(2025, 12, 31))
        .is_empty());
    assert!(cal
        .sessions_between(d(2023, 1, 1), d(2023, 12, 31))
        .is_empty());
}
