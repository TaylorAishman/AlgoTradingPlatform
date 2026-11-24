//! The error type for `pt-calendar`.

use chrono::NaiveDate;
use thiserror::Error;

/// Errors raised while building a trading calendar.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CalendarError {
    #[error("calendar has no sessions")]
    Empty,
    #[error("calendar sessions are not sorted")]
    NotSorted,
    #[error("duplicate session {0}")]
    Duplicate(NaiveDate),
}
