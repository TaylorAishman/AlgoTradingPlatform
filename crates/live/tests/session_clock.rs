//! Tests that the trade session and expected last session come from the broker clock.

use chrono::{DateTime, FixedOffset, NaiveDate};
use pt_alpaca::reference::WireClock;
use pt_calendar::calendar::TradingCalendar;
use pt_live::snapshot::{expected_last_session, trade_session};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

fn t(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).expect("valid instant")
}

fn calendar_2024() -> TradingCalendar {
    TradingCalendar::new(vec![
        d(2024, 2, 26),
        d(2024, 2, 27),
        d(2024, 2, 28),
        d(2024, 2, 29),
        d(2024, 3, 1),
        d(2024, 3, 4),
        d(2024, 3, 5),
    ])
    .expect("valid calendar")
}

fn pre_open() -> WireClock {
    WireClock {
        timestamp: t("2024-03-01T08:00:00-05:00"),
        is_open: false,
        next_open: t("2024-03-01T09:30:00-05:00"),
        next_close: t("2024-03-01T16:00:00-05:00"),
    }
}

fn open() -> WireClock {
    WireClock {
        timestamp: t("2024-03-01T10:00:00-05:00"),
        is_open: true,
        next_open: t("2024-03-04T09:30:00-05:00"),
        next_close: t("2024-03-01T16:00:00-05:00"),
    }
}

fn post_close() -> WireClock {
    WireClock {
        timestamp: t("2024-03-01T17:30:00-05:00"),
        is_open: false,
        next_open: t("2024-03-04T09:30:00-05:00"),
        next_close: t("2024-03-04T16:00:00-05:00"),
    }
}

// R-282
#[test]
fn pre_open_clock_trades_today_and_data_is_complete_through_yesterday() {
    let c = pre_open();
    assert_eq!(trade_session(&c), d(2024, 3, 1));
    assert_eq!(
        expected_last_session(&c, &calendar_2024()),
        Some(d(2024, 2, 29))
    );
}

// R-282
#[test]
fn open_clock_trades_today_and_data_is_complete_through_yesterday() {
    let c = open();
    assert_eq!(trade_session(&c), d(2024, 3, 1));
    assert_eq!(
        expected_last_session(&c, &calendar_2024()),
        Some(d(2024, 2, 29))
    );
}

// R-282
#[test]
fn post_close_clock_trades_next_session_and_data_is_complete_through_today() {
    let c = post_close();
    assert_eq!(trade_session(&c), d(2024, 3, 4));
    assert_eq!(
        expected_last_session(&c, &calendar_2024()),
        Some(d(2024, 3, 1))
    );
}
