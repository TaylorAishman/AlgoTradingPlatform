//! Session dates taken from the broker clock, and equity snapshots of the paper book.

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::halts::set_halt;
use chrono::NaiveDate;
use pt_alpaca::reference::WireClock;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Adjustment;
use pt_ops::alert::Level;
use pt_risk::halts::{check_equity, HaltReason};
use pt_risk::pretrade::RiskLimits;
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
/// snapshot yet it starts at the date of the first ledger entry. It stops at
/// the first session lacking a raw close for a held symbol, so no later session
/// is stored either. More than one new snapshot records a `missed_run` warning.
/// Returns how many snapshots it stored.
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
        let mut missing = false;
        for (symbol, qty) in ctx.store.positions_at(Book::Paper, date)? {
            let bars = ctx.store.bars_asof(
                &symbol,
                Adjustment::Raw,
                &ctx.cfg.data.feed,
                date,
                date,
                None,
            )?;
            let Some(bar) = bars.first() else {
                missing = true;
                break;
            };
            positions_value += bar.close.value() * Decimal::from(qty);
        }
        if missing {
            break;
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
    if stored > 1 {
        ctx.alerter.alert(
            ctx.store,
            Level::Warn,
            "missed_run",
            &format!("{stored} sessions were snapshotted at once; runs were missed"),
            None,
            ctx.clock.now_utc(),
        );
    }
    Ok(stored)
}

/// Checks the stored paper equity series against the configured halt limits
/// with `check_equity`. A breach sets one halt and a critical `halt_set` alert;
/// while a halt for the same reason is still active nothing more is added.
/// Returns the breach found, if any.
pub fn check_halts(ctx: &LiveContext<'_>) -> Result<Option<HaltReason>, LiveError> {
    let equity: Vec<Decimal> = ctx
        .store
        .equity_series(Book::Paper)?
        .iter()
        .map(|s| s.equity)
        .collect();
    let limits = RiskLimits {
        max_order_notional: ctx.cfg.risk.max_order_notional,
        max_weight: ctx.cfg.sizing.max_weight,
        max_gross: ctx.cfg.sizing.max_gross,
        daily_loss_limit: ctx.cfg.risk.daily_loss_limit,
        max_drawdown: ctx.cfg.risk.max_drawdown,
    };
    let Some(reason) = check_equity(&equity, &limits) else {
        return Ok(None);
    };
    let name = match reason {
        HaltReason::DailyLoss => "daily_loss",
        HaltReason::Drawdown => "drawdown",
    };
    let detail = format!("equity series breached the {name} limit");
    set_halt(ctx, name, &detail)?;
    Ok(Some(reason))
}
