//! Tests for the P&L returns computed from the snapshot series (R-355).

use chrono::NaiveDate;
use pt_ops::pnl::pnl;
use rust_decimal::Decimal;
use std::str::FromStr;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn series(rows: &[(NaiveDate, &str)]) -> Vec<(NaiveDate, Decimal)> {
    rows.iter().map(|(date, v)| (*date, dec(v))).collect()
}

// R-355: the worked example from the acceptance check.
#[test]
fn day_mtd_ytd_inception_match_the_worked_example() {
    let s = series(&[
        (d(2025, 12, 31), "100000"),
        (d(2026, 1, 30), "102000"),
        (d(2026, 2, 26), "101000"),
        (d(2026, 2, 27), "101505"),
    ]);
    let p = pnl(&s, d(2026, 2, 27)).expect("snapshot exists for the date");
    assert_eq!(p.day.map(|v| v.round_dp(6)), Some(dec("0.005")));
    assert_eq!(p.mtd.map(|v| v.round_dp(6)), Some(dec("-0.004853")));
    assert_eq!(p.ytd.map(|v| v.round_dp(6)), Some(dec("0.01505")));
    assert_eq!(p.inception.map(|v| v.round_dp(6)), Some(dec("0.01505")));
}

// R-355: month-to-date is measured from the last snapshot of the previous month.
#[test]
fn mtd_uses_last_snapshot_of_previous_month() {
    let s = series(&[
        (d(2026, 1, 28), "90"),
        (d(2026, 1, 30), "100"),
        (d(2026, 2, 2), "105"),
        (d(2026, 2, 3), "110"),
    ]);
    let p = pnl(&s, d(2026, 2, 3)).unwrap();
    assert_eq!(p.mtd.map(|v| v.round_dp(6)), Some(dec("0.1")));
}

// R-355: year-to-date is measured from the last snapshot of the previous year.
#[test]
fn ytd_uses_last_snapshot_of_previous_year() {
    let s = series(&[
        (d(2025, 12, 30), "50"),
        (d(2025, 12, 31), "100"),
        (d(2026, 1, 2), "120"),
        (d(2026, 3, 2), "130"),
    ]);
    let p = pnl(&s, d(2026, 3, 2)).unwrap();
    assert_eq!(p.ytd.map(|v| v.round_dp(6)), Some(dec("0.3")));
    assert_eq!(p.inception.map(|v| v.round_dp(6)), Some(dec("1.6")));
}

// R-355: with no snapshot in the previous month or year, both use the first snapshot.
#[test]
fn mtd_and_ytd_fall_back_to_first_snapshot() {
    let s = series(&[(d(2026, 2, 2), "100"), (d(2026, 2, 3), "110")]);
    let p = pnl(&s, d(2026, 2, 3)).unwrap();
    assert_eq!(p.mtd.map(|v| v.round_dp(6)), Some(dec("0.1")));
    assert_eq!(p.ytd.map(|v| v.round_dp(6)), Some(dec("0.1")));
    assert_eq!(p.inception.map(|v| v.round_dp(6)), Some(dec("0.1")));
    assert_eq!(p.day.map(|v| v.round_dp(6)), Some(dec("0.1")));
}

// R-355: the report is for the asked date, not the latest snapshot.
#[test]
fn asof_in_the_middle_of_the_series_ignores_later_snapshots() {
    let s = series(&[
        (d(2026, 2, 2), "100"),
        (d(2026, 2, 3), "110"),
        (d(2026, 2, 4), "50"),
    ]);
    let p = pnl(&s, d(2026, 2, 3)).unwrap();
    assert_eq!(p.day.map(|v| v.round_dp(6)), Some(dec("0.1")));
    assert_eq!(p.inception.map(|v| v.round_dp(6)), Some(dec("0.1")));
}
