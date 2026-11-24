//! The trading calendar: an ordered list of session dates.

use chrono::{Datelike, NaiveDate};

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

    /// Sessions in `start..=end`, inclusive at both ends. Empty when `start > end`.
    pub fn sessions_between(&self, start: NaiveDate, end: NaiveDate) -> &[NaiveDate] {
        let lo = self.sessions.partition_point(|&s| s < start);
        let hi = self.sessions.partition_point(|&s| s <= end);
        if lo >= hi {
            return &[];
        }
        &self.sessions[lo..hi]
    }

    /// Calendar days from `date` to the next session after it, or `None` past the end.
    pub fn calendar_days_to_next(&self, date: NaiveDate) -> Option<i64> {
        self.next_session(date).map(|n| (n - date).num_days())
    }

    /// Sessions in `start..=end` whose next session falls in a different month.
    /// The last session of the calendar is never one, since its successor is unknown.
    pub fn month_end_sessions(&self, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
        self.sessions_between(start, end)
            .iter()
            .copied()
            .filter(|&s| {
                self.next_session(s)
                    .is_some_and(|n| (n.year(), n.month()) != (s.year(), s.month()))
            })
            .collect()
    }

    /// The last month-end session of the calendar month `months` months before the month of
    /// `date`, or `None` when the calendar holds no such session.
    pub fn month_end_back(&self, date: NaiveDate, months: u32) -> Option<NaiveDate> {
        let index = i64::from(date.year()) * 12 + i64::from(date.month0()) - i64::from(months);
        let year = i32::try_from(index.div_euclid(12)).ok()?;
        let month = u32::try_from(index.rem_euclid(12)).ok()? + 1;
        let first = NaiveDate::from_ymd_opt(year, month, 1)?;
        let last = first
            .checked_add_months(chrono::Months::new(1))?
            .pred_opt()?;
        self.month_end_sessions(first, last).last().copied()
    }
}
