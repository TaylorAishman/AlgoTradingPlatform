//! Tests that each P&L value is absent when its own base equity is zero (R-356).

use chrono::NaiveDate;
use pt_ops::pnl::pnl;
use rust_decimal::Decimal;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

// R-356: with a zero first and previous equity, all four values are None.
#[test]
fn zero_base_for_every_value_makes_all_four_none() {
    let s = vec![
        (d(2026, 3, 2), Decimal::ZERO),
        (d(2026, 3, 3), Decimal::from(100)),
    ];
    let p = pnl(&s, d(2026, 3, 3)).expect("snapshot exists for the date");
    assert_eq!(p.day, None);
    assert_eq!(p.mtd, None);
    assert_eq!(p.ytd, None);
    assert_eq!(p.inception, None);
}

// R-356: only the values whose base is zero are absent; the others are kept.
#[test]
fn only_the_value_with_a_zero_base_is_none() {
    let s = vec![
        (d(2026, 2, 27), Decimal::from(100)),
        (d(2026, 3, 2), Decimal::ZERO),
        (d(2026, 3, 3), Decimal::from(50)),
    ];
    let p = pnl(&s, d(2026, 3, 3)).expect("snapshot exists for the date");
    let half = Some(Decimal::new(-5, 1));
    assert_eq!(p.day, None);
    assert_eq!(p.mtd, half);
    assert_eq!(p.ytd, half);
    assert_eq!(p.inception, half);
}

// R-356: a zero equity today with a positive base is a real -100% return, not absent.
#[test]
fn zero_equity_today_is_a_total_loss_not_absent() {
    let s = vec![
        (d(2026, 3, 2), Decimal::from(100)),
        (d(2026, 3, 3), Decimal::ZERO),
    ];
    let p = pnl(&s, d(2026, 3, 3)).expect("snapshot exists for the date");
    let all = Some(Decimal::NEGATIVE_ONE);
    assert_eq!(p.day, all);
    assert_eq!(p.mtd, all);
    assert_eq!(p.ytd, all);
    assert_eq!(p.inception, all);
}
