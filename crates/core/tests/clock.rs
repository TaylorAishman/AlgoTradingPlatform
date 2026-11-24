//! Tests for the injectable clock (R-019).

use chrono::{TimeZone, Utc};
use pt_core::clock::{Clock, FixedClock, SystemClock};

// R-019: FixedClock(t).now_utc() == t for a chosen instant.
#[test]
fn fixed_clock_returns_the_instant_it_was_given() {
    let t = Utc.with_ymd_and_hms(2024, 3, 28, 14, 30, 5).unwrap();
    assert_eq!(FixedClock(t).now_utc(), t);
}

// R-019: the fixed clock does not advance between calls.
#[test]
fn fixed_clock_is_stable_across_calls() {
    let t = Utc.with_ymd_and_hms(2031, 12, 31, 23, 59, 59).unwrap();
    let clock = FixedClock(t);
    assert_eq!(clock.now_utc(), clock.now_utc());
}

// R-019: SystemClock.now_utc() is later than 2026-01-01T00:00:00Z.
#[test]
fn system_clock_is_after_2026_01_01() {
    let floor = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    assert!(SystemClock.now_utc() > floor);
}

// R-019: time is injectable, so both clocks work behind the trait.
#[test]
fn clocks_are_usable_as_trait_objects() {
    let t = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
    let clocks: Vec<Box<dyn Clock>> = vec![Box::new(FixedClock(t)), Box::new(SystemClock)];
    assert_eq!(clocks[0].now_utc(), t);
    assert!(clocks[1].now_utc() > t);
}
