//! Comparing two equity series with a tolerance (R-223).

use chrono::NaiveDate;
use pt_vector::error::VectorError;
use pt_vector::vector::compare;
use rust_decimal::Decimal;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

fn dates() -> Vec<NaiveDate> {
    vec![d(2), d(3), d(4)]
}

fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

fn base() -> Vec<Decimal> {
    vec![dec(100), dec(101), dec(102)]
}

fn drifted() -> Vec<Decimal> {
    vec![dec(100), Decimal::new(101004, 3), Decimal::new(10202, 2)]
}

// R-223
#[test]
fn equal_series_have_no_difference() {
    let r = compare(&dates(), &base(), &base(), Decimal::new(1, 2)).unwrap();
    assert!(r.equal);
    assert_eq!(r.max_abs_diff, Decimal::ZERO);
    assert_eq!(r.first_diff_date, None);
}

// R-223
#[test]
fn difference_above_tolerance_is_reported_with_first_date() {
    let r = compare(&dates(), &base(), &drifted(), Decimal::new(1, 2)).unwrap();
    assert!(!r.equal);
    assert_eq!(r.max_abs_diff, Decimal::new(2, 2));
    assert_eq!(r.first_diff_date, Some(d(4)));
}

// R-223
#[test]
fn difference_within_tolerance_is_equal_but_keeps_max_diff() {
    let r = compare(&dates(), &base(), &drifted(), Decimal::new(5, 2)).unwrap();
    assert!(r.equal);
    assert_eq!(r.max_abs_diff, Decimal::new(2, 2));
    assert_eq!(r.first_diff_date, None);
}

// R-223
#[test]
fn difference_is_absolute_and_first_date_is_the_earliest_breach() {
    let a = vec![dec(100), dec(105), dec(90)];
    let b = vec![dec(100), dec(103), dec(100)];
    let r = compare(&dates(), &a, &b, Decimal::ONE).unwrap();
    assert!(!r.equal);
    assert_eq!(r.max_abs_diff, dec(10));
    assert_eq!(r.first_diff_date, Some(d(3)));
}

// R-223
#[test]
fn different_lengths_are_a_length_mismatch() {
    let short = vec![dec(100), dec(101)];
    assert_eq!(
        compare(&dates(), &base(), &short, Decimal::ONE).unwrap_err(),
        VectorError::LengthMismatch
    );
    assert_eq!(
        compare(&dates(), &short, &base(), Decimal::ONE).unwrap_err(),
        VectorError::LengthMismatch
    );
}
