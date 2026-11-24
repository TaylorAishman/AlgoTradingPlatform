mod common;

use common::{calendar_2024, d};

// R-025
#[test]
fn one_month_back_from_a_month_end_is_the_previous_month_end() {
    assert_eq!(
        calendar_2024().month_end_back(d(2024, 11, 29), 1),
        Some(d(2024, 10, 31))
    );
}

// R-025
#[test]
fn eight_months_back_from_mid_month_is_the_last_session_of_that_month() {
    assert_eq!(
        calendar_2024().month_end_back(d(2024, 11, 15), 8),
        Some(d(2024, 3, 28))
    );
}

// R-025
#[test]
fn lookback_before_the_first_month_of_the_calendar_is_none() {
    assert_eq!(calendar_2024().month_end_back(d(2024, 11, 29), 12), None);
}

// R-025
#[test]
fn lookback_reaching_the_first_month_of_the_calendar_finds_its_last_session() {
    assert_eq!(
        calendar_2024().month_end_back(d(2024, 12, 31), 11),
        Some(d(2024, 1, 31))
    );
}

// R-025
#[test]
fn one_month_back_from_the_last_session_of_the_calendar() {
    assert_eq!(
        calendar_2024().month_end_back(d(2024, 12, 31), 1),
        Some(d(2024, 11, 29))
    );
}
