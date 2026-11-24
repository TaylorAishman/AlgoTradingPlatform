mod common;

use common::{calendar_2024, d, sessions_2024};

// R-021
#[test]
fn next_session_skips_good_friday() {
    assert_eq!(
        calendar_2024().next_session(d(2024, 3, 28)),
        Some(d(2024, 4, 1))
    );
}

// R-021
#[test]
fn prev_session_skips_good_friday() {
    assert_eq!(
        calendar_2024().prev_session(d(2024, 4, 1)),
        Some(d(2024, 3, 28))
    );
}

// R-021
#[test]
fn next_session_from_a_non_session_date_skips_weekend() {
    assert_eq!(
        calendar_2024().next_session(d(2024, 3, 30)),
        Some(d(2024, 4, 1))
    );
}

// R-021
#[test]
fn prev_session_from_a_non_session_date_skips_weekend() {
    assert_eq!(
        calendar_2024().prev_session(d(2024, 3, 31)),
        Some(d(2024, 3, 28))
    );
}

// R-021
#[test]
fn next_session_after_last_session_is_none() {
    assert_eq!(calendar_2024().next_session(d(2024, 12, 31)), None);
    assert_eq!(calendar_2024().next_session(d(2025, 6, 1)), None);
}

// R-021
#[test]
fn prev_session_before_first_session_is_none() {
    assert_eq!(calendar_2024().prev_session(d(2024, 1, 2)), None);
    assert_eq!(calendar_2024().prev_session(d(2023, 6, 1)), None);
}

// R-021
#[test]
fn lookups_are_strict_for_session_dates() {
    let cal = calendar_2024();
    assert_eq!(cal.next_session(d(2024, 1, 2)), Some(d(2024, 1, 3)));
    assert_eq!(cal.prev_session(d(2024, 1, 3)), Some(d(2024, 1, 2)));
}

// R-021
#[test]
fn lookups_before_first_and_after_last_reach_the_ends() {
    let cal = calendar_2024();
    assert_eq!(cal.next_session(d(2023, 6, 1)), Some(d(2024, 1, 2)));
    assert_eq!(cal.prev_session(d(2025, 6, 1)), Some(d(2024, 12, 31)));
}

// R-021
#[test]
fn good_friday_is_not_a_session() {
    assert!(!calendar_2024().is_session(d(2024, 3, 29)));
}

// R-021
#[test]
fn next_session_never_returns_a_non_session_over_whole_year() {
    let cal = calendar_2024();
    let s = sessions_2024();
    for w in s.windows(2) {
        assert_eq!(cal.next_session(w[0]), Some(w[1]));
        assert_eq!(cal.prev_session(w[1]), Some(w[0]));
    }
}
