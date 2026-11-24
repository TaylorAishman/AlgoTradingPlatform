//! Tests that P&L is absent rather than wrong without a snapshot (R-356).

use chrono::NaiveDate;
use pt_ops::pnl::pnl;
use rust_decimal::Decimal;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

// R-356: no snapshot for the date gives None, even with earlier and later ones.
#[test]
fn no_snapshot_for_the_date_is_none() {
    let s = vec![
        (d(2026, 2, 27), Decimal::from(100)),
        (d(2026, 3, 3), Decimal::from(110)),
    ];
    assert_eq!(pnl(&s, d(2026, 3, 2)), None);
}

// R-356: an empty series gives None.
#[test]
fn empty_series_is_none() {
    assert_eq!(pnl(&[], d(2026, 3, 2)), None);
}

// R-356: a date after the last snapshot is None, not the stale value.
#[test]
fn date_after_the_last_snapshot_is_none() {
    let s = vec![(d(2026, 2, 27), Decimal::from(100))];
    assert_eq!(pnl(&s, d(2026, 3, 2)), None);
}

// R-356: a one-element series at its own date has all four values 0.
#[test]
fn single_snapshot_at_its_date_is_all_zero() {
    let s = vec![(d(2026, 3, 2), Decimal::from(100))];
    let p = pnl(&s, d(2026, 3, 2)).expect("snapshot exists");
    assert_eq!(p.day, Decimal::ZERO);
    assert_eq!(p.mtd, Decimal::ZERO);
    assert_eq!(p.ytd, Decimal::ZERO);
    assert_eq!(p.inception, Decimal::ZERO);
}
