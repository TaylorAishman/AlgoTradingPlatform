//! R-058: the store holds the one calendar.

use chrono::{Datelike, NaiveDate, Weekday};
use pt_store::db::Store;
use pt_store::error::StoreError;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// 252 distinct weekdays of 2024, in order.
fn sessions_2024() -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut day = d(2024, 1, 1);
    while day.year() == 2024 && out.len() < 252 {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(day);
        }
        day = day.succ_opt().unwrap();
    }
    out
}

fn count(s: &Store) -> usize {
    s.calendar()
        .unwrap()
        .sessions_between(d(2000, 1, 1), d(2100, 1, 1))
        .len()
}

#[test]
fn put_sessions_then_calendar_yields_all_sessions() {
    // R-058
    let s = Store::open_in_memory().unwrap();
    let sessions = sessions_2024();
    assert_eq!(sessions.len(), 252);
    s.put_sessions(&sessions).unwrap();
    let cal = s.calendar().unwrap();
    assert_eq!(count(&s), 252);
    assert!(cal.is_session(sessions[0]));
    assert!(cal.is_session(*sessions.last().unwrap()));
    assert!(!cal.is_session(d(2024, 1, 6)));
}

#[test]
fn put_sessions_again_with_a_new_date_adds_it_without_error() {
    // R-058
    let s = Store::open_in_memory().unwrap();
    let mut sessions = sessions_2024();
    s.put_sessions(&sessions).unwrap();
    sessions.push(d(2025, 1, 2));
    s.put_sessions(&sessions).unwrap();
    assert_eq!(count(&s), 253);
    assert!(s.calendar().unwrap().is_session(d(2025, 1, 2)));
}

#[test]
fn calendar_on_an_empty_table_is_not_found() {
    // R-058
    let s = Store::open_in_memory().unwrap();
    match s.calendar() {
        Err(StoreError::NotFound(_)) => {}
        Err(e) => panic!("expected NotFound, got {e:?}"),
        Ok(_) => panic!("expected NotFound, got a calendar"),
    }
}
