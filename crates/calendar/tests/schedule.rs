mod common;

use chrono::NaiveDate;
use common::{calendar_2024, d};
use pt_calendar::schedule::{monthly_schedule, Rebalance};

fn pairs(v: Vec<Rebalance>) -> Vec<(NaiveDate, NaiveDate)> {
    v.into_iter()
        .map(|r| (r.signal_date, r.trade_date))
        .collect()
}

// R-026
#[test]
fn schedule_pairs_each_month_end_with_the_next_session() {
    let got = pairs(monthly_schedule(
        &calendar_2024(),
        d(2024, 1, 1),
        d(2024, 4, 30),
    ));
    assert_eq!(
        got,
        vec![
            (d(2024, 1, 31), d(2024, 2, 1)),
            (d(2024, 2, 29), d(2024, 3, 1)),
            (d(2024, 3, 28), d(2024, 4, 1)),
        ]
    );
}

// R-026
#[test]
fn pair_whose_trade_date_falls_after_to_is_not_included() {
    // 2024-04-30 is a month end, but its trade date 2024-05-01 is after `to`.
    let got = pairs(monthly_schedule(
        &calendar_2024(),
        d(2024, 1, 1),
        d(2024, 4, 30),
    ));
    assert!(got.iter().all(|(_, t)| *t <= d(2024, 4, 30)));
    assert!(!got.iter().any(|(s, _)| *s == d(2024, 4, 30)));
}

// R-026
#[test]
fn trade_date_equal_to_to_is_included() {
    let got = pairs(monthly_schedule(
        &calendar_2024(),
        d(2024, 1, 1),
        d(2024, 2, 1),
    ));
    assert_eq!(got, vec![(d(2024, 1, 31), d(2024, 2, 1))]);
}

// R-026
#[test]
fn signal_dates_before_from_are_not_included() {
    let got = pairs(monthly_schedule(
        &calendar_2024(),
        d(2024, 2, 1),
        d(2024, 4, 30),
    ));
    assert_eq!(
        got,
        vec![
            (d(2024, 2, 29), d(2024, 3, 1)),
            (d(2024, 3, 28), d(2024, 4, 1)),
        ]
    );
}

// R-026
#[test]
fn range_without_a_month_end_gives_an_empty_schedule() {
    let got = monthly_schedule(&calendar_2024(), d(2024, 2, 1), d(2024, 2, 28));
    assert!(got.is_empty());
}
