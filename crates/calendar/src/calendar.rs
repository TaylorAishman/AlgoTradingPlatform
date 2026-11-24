//! The trading calendar: an ordered list of session dates.

use chrono::NaiveDate;

use crate::error::CalendarError;

/// A strictly ascending list of trading sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradingCalendar {
    sessions: Vec<NaiveDate>,
}

impl TradingCalendar {
    /// Build a calendar from session dates, which must be non-empty and strictly ascending.
    pub fn new(sessions: Vec<NaiveDate>) -> Result<Self, CalendarError> {
        if sessions.is_empty() {
            return Err(CalendarError::Empty);
        }
        for pair in sessions.windows(2) {
            if pair[0] == pair[1] {
                return Err(CalendarError::Duplicate(pair[0]));
            }
            if pair[0] > pair[1] {
                return Err(CalendarError::NotSorted);
            }
        }
        Ok(Self { sessions })
    }

    /// True when `date` is a listed session.
    pub fn is_session(&self, date: NaiveDate) -> bool {
        self.sessions.binary_search(&date).is_ok()
    }

    /// The first session strictly after `date`, or `None` past the end of the calendar.
    pub fn next_session(&self, date: NaiveDate) -> Option<NaiveDate> {
        let i = self.sessions.partition_point(|&s| s <= date);
        self.sessions.get(i).copied()
    }

    /// The last session strictly before `date`, or `None` before the start of the calendar.
    pub fn prev_session(&self, date: NaiveDate) -> Option<NaiveDate> {
        let i = self.sessions.partition_point(|&s| s < date);
        i.checked_sub(1).map(|j| self.sessions[j])
    }
}
