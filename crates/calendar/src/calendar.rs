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
}
