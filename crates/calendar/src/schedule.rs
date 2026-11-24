//! Monthly rebalance schedule.

use chrono::NaiveDate;

use crate::calendar::TradingCalendar;

/// A rebalance: the signal is computed at `signal_date` (a month-end session)
/// and traded on `trade_date` (the next session).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rebalance {
    pub signal_date: NaiveDate,
    pub trade_date: NaiveDate,
}

/// Pair each month-end session in `[from, to]` with the next session,
/// keeping only pairs whose trade date is on or before `to`.
pub fn monthly_schedule(cal: &TradingCalendar, from: NaiveDate, to: NaiveDate) -> Vec<Rebalance> {
    cal.month_end_sessions(from, to)
        .into_iter()
        .filter_map(|signal_date| {
            let trade_date = cal.next_session(signal_date)?;
            (trade_date <= to).then_some(Rebalance {
                signal_date,
                trade_date,
            })
        })
        .collect()
}
