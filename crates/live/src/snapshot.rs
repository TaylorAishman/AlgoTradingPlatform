//! Session dates taken from the broker clock.

use chrono::NaiveDate;
use pt_alpaca::reference::WireClock;
use pt_calendar::calendar::TradingCalendar;

/// The session the next orders trade in: today's while the market is open or
/// not yet open, otherwise the next one.
pub fn trade_session(clock: &WireClock) -> NaiveDate {
    if clock.is_open {
        clock.next_close.date_naive()
    } else {
        clock.next_open.date_naive()
    }
}

/// The last session whose bars must be complete: the session before the trade session.
pub fn expected_last_session(clock: &WireClock, calendar: &TradingCalendar) -> Option<NaiveDate> {
    calendar.prev_session(trade_session(clock))
}
