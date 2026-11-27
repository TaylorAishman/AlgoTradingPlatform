//! `slot_due`: weekday slots at 08:00, 09:45 and 17:30 New York time.

use chrono::{DateTime, Utc};
use pt_live::slots::slot_due;

fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

// R-514
#[test]
fn slot_at_0945_new_york_is_due_exactly_when_it_has_passed() {
    let last = Some(t("2026-03-06T13:00:00Z"));
    assert!(!slot_due(last, t("2026-03-06T14:44:59Z")));
    assert!(slot_due(last, t("2026-03-06T14:45:00Z")));
}

// R-514
#[test]
fn slot_at_1730_new_york_is_due_exactly_when_it_has_passed() {
    let last = Some(t("2026-03-06T14:45:00Z"));
    assert!(!slot_due(last, t("2026-03-06T22:29:59Z")));
    assert!(slot_due(last, t("2026-03-06T22:30:00Z")));
}

// R-514
#[test]
fn no_time_has_passed_means_not_due() {
    let now = t("2026-03-06T14:45:00Z");
    assert!(!slot_due(Some(now), now));
}

// R-515
#[test]
fn slots_follow_the_spring_daylight_saving_change() {
    let last = Some(t("2026-03-06T22:30:00Z"));
    assert!(!slot_due(last, t("2026-03-09T11:59:59Z")));
    assert!(slot_due(last, t("2026-03-09T12:00:00Z")));
}

// R-515
#[test]
fn slots_follow_the_autumn_daylight_saving_change() {
    let last = Some(t("2026-10-30T21:30:00Z"));
    assert!(!slot_due(last, t("2026-11-02T12:59:59Z")));
    assert!(slot_due(last, t("2026-11-02T13:00:00Z")));
}

// R-516
#[test]
fn weekends_have_no_slot() {
    let last = Some(t("2026-03-06T22:30:00Z"));
    assert!(!slot_due(last, t("2026-03-08T20:00:00Z")));
}

// R-516
#[test]
fn a_first_run_is_always_due() {
    assert!(slot_due(None, t("2026-03-06T15:00:00Z")));
    assert!(slot_due(None, t("2026-03-08T20:00:00Z")));
}
