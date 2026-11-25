//! Session dates taken from the broker clock, and equity snapshots of the paper book.

use crate::context::LiveContext;
use crate::error::LiveError;
use chrono::NaiveDate;
use pt_alpaca::reference::WireClock;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Adjustment;
use pt_store::trading::{Book, EquitySnapshot};
use rust_decimal::Decimal;

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

/// Stores a snapshot for every session after the last one stored, up to and
/// including `through`, valuing positions at the stored raw close. With no
/// snapshot yet it starts at the date of the first ledger entry. Returns how
/// many snapshots it stored.
pub fn snapshot_equity(ctx: &LiveContext<'_>, through: NaiveDate) -> Result<usize, LiveError> {
    let start = match ctx.store.equity_series(Book::Paper)?.last() {
        Some(last) => match last.date.succ_opt() {
            Some(next) => next,
            None => return Ok(0),
        },
        None => match ctx.store.ledger(Book::Paper)?.iter().map(|e| e.date).min() {
            Some(first) => first,
            None => return Ok(0),
        },
    };
    let calendar = ctx.store.calendar()?;
    let mut stored = 0;
    for &date in calendar.sessions_between(start, through) {
        let cash = ctx.store.cash_at(Book::Paper, date)?;
        let mut positions_value = Decimal::ZERO;
        for (symbol, qty) in ctx.store.positions_at(Book::Paper, date)? {
            let bars = ctx.store.bars_asof(
                &symbol,
                Adjustment::Raw,
                &ctx.cfg.data.feed,
                date,
                date,
                None,
            )?;
            let bar = bars.first().ok_or_else(|| {
                LiveError::Invalid(format!("no raw close for {symbol} on {date}"))
            })?;
            positions_value += bar.close.value() * Decimal::from(qty);
        }
        ctx.store.put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date,
            cash,
            positions_value,
            equity: cash + positions_value,
            recorded_at: ctx.clock.now_utc(),
        })?;
        stored += 1;
    }
    Ok(stored)
}
