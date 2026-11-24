mod common;

use common::{calendar_2024, d, sessions_2024};

// R-023
#[test]
fn friday_before_a_weekend_is_three_days_from_monday() {
    assert_eq!(
        calendar_2024().calendar_days_to_next(d(2024, 3, 22)),
        Some(3)
    );
}

// R-023
#[test]
fn thursday_before_good_friday_is_four_days_from_monday() {
    assert_eq!(
        calendar_2024().calendar_days_to_next(d(2024, 3, 28)),
        Some(4)
    );
}

// R-023
#[test]
fn consecutive_weekdays_are_one_day_apart() {
    assert_eq!(
        calendar_2024().calendar_days_to_next(d(2024, 1, 2)),
        Some(1)
    );
}

// R-023
#[test]
fn last_session_has_no_next_session() {
    assert_eq!(calendar_2024().calendar_days_to_next(d(2024, 12, 31)), None);
}

// R-023
#[test]
fn a_non_session_date_counts_to_the_next_session() {
    assert_eq!(
        calendar_2024().calendar_days_to_next(d(2024, 3, 30)),
        Some(2)
    );
}

// R-023
#[test]
fn every_session_gap_matches_the_date_difference() {
    let cal = calendar_2024();
    for w in sessions_2024().windows(2) {
        assert_eq!(
            cal.calendar_days_to_next(w[0]),
            Some((w[1] - w[0]).num_days())
        );
    }
}
