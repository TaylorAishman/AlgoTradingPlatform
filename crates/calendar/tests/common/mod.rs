//! Shared fixtures: the 2024 NYSE session list (252 sessions).
#![allow(dead_code)]

use chrono::{Datelike, NaiveDate, Weekday};
use pt_calendar::calendar::TradingCalendar;

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

/// Weekdays of 2024 minus the ten NYSE full-day holidays.
pub fn sessions_2024() -> Vec<NaiveDate> {
    let holidays = [
        d(2024, 1, 1),
        d(2024, 1, 15),
        d(2024, 2, 19),
        d(2024, 3, 29),
        d(2024, 5, 27),
        d(2024, 6, 19),
        d(2024, 7, 4),
        d(2024, 9, 2),
        d(2024, 11, 28),
        d(2024, 12, 25),
    ];
    let mut out = Vec::new();
    let mut day = d(2024, 1, 1);
    while day.year() == 2024 {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) && !holidays.contains(&day) {
            out.push(day);
        }
        day = day.succ_opt().expect("next day");
    }
    out
}

pub fn calendar_2024() -> TradingCalendar {
    TradingCalendar::new(sessions_2024()).expect("2024 calendar is valid")
}
