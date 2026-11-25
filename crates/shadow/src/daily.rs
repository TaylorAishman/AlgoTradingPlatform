//! The daily shadow simulation: one session replayed from the paper book.

use crate::error::ShadowError;
use chrono::NaiveDate;
use pt_config::config::Config;
use pt_core::money::{Money, Price};
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Fill, Symbol};
use pt_engine::book::Book as SimBook;
use pt_engine::cost::{borrow_cost, fill_cost};
use pt_engine::error::EngineError;
use pt_store::db::Store;
use pt_store::shadow::ShadowDay;
use pt_store::trading::{Book, EquitySnapshot};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Orders that reached the broker and so take part in the shadow day.
const SENT: [OrderState; 5] = [
    OrderState::Submitted,
    OrderState::PartiallyFilled,
    OrderState::Filled,
    OrderState::Canceled,
    OrderState::Expired,
];

/// Compare the paper return of `date` with a shadow simulation of the same day.
///
/// The shadow book starts from the paper ledger at the previous session's
/// close, fills every order sent for `date` completely at the raw open with
/// `cost.shadow`, charges borrow on shorts and marks at the raw close. The
/// paper return comes from the paper equity snapshots of the previous session
/// and `date`. `diff` is paper minus shadow. A date that is not a session, has
/// no previous session, or lacks either snapshot is `NotFound`.
pub fn shadow_day(store: &Store, cfg: &Config, date: NaiveDate) -> Result<ShadowDay, ShadowError> {
    let calendar = store.calendar()?;
    if !calendar.is_session(date) {
        return Err(ShadowError::NotFound(format!("{date} is not a session")));
    }
    let prev = calendar
        .prev_session(date)
        .ok_or_else(|| ShadowError::NotFound(format!("no session before {date}")))?;

    let snapshots = store.equity_series(Book::Paper)?;
    let paper_start = snapshot_equity(&snapshots, prev)?;
    let paper_end = snapshot_equity(&snapshots, date)?;

    let mut book = SimBook::new(store.cash_at(Book::Paper, prev)?);
    book.positions = store.positions_at(Book::Paper, prev)?;
    let shadow_start = book.equity(&raw_closes(store, cfg, &book, prev)?)?;

    let mut orders: Vec<_> = store
        .orders_in_states(&SENT)?
        .into_iter()
        .filter(|o| o.trade_date == date)
        .collect();
    orders.sort_by(|a, b| a.client_order_id.cmp(&b.client_order_id));
    for order in orders {
        let open = raw_bar(store, cfg, &order.symbol, date)?.open;
        let cost = fill_cost(&cfg.cost.shadow, order.qty, open, None)?;
        book.apply_fill(&Fill {
            symbol: order.symbol,
            side: order.side,
            qty: order.qty,
            price: open,
            cost,
        });
    }

    let closes = raw_closes(store, cfg, &book, date)?;
    let short_value: Decimal = book
        .positions
        .iter()
        .filter(|(_, qty)| **qty < 0)
        .map(|(symbol, qty)| closes[symbol].notional(*qty).0.abs())
        .sum();
    let days = calendar.calendar_days_to_next(date).unwrap_or(1);
    book.cash -= borrow_cost(&cfg.cost.shadow, Money(short_value), days).0;
    let shadow_end = book.equity(&closes)?;

    let paper_return = simple_return(paper_start, paper_end, "paper", prev)?;
    let shadow_return = simple_return(shadow_start, shadow_end, "shadow", prev)?;
    Ok(ShadowDay {
        date,
        paper_return: to_f64(paper_return),
        shadow_return: to_f64(shadow_return),
        diff: to_f64(paper_return - shadow_return),
    })
}

fn snapshot_equity(snapshots: &[EquitySnapshot], date: NaiveDate) -> Result<Decimal, ShadowError> {
    snapshots
        .iter()
        .find(|s| s.date == date)
        .map(|s| s.equity)
        .ok_or_else(|| ShadowError::NotFound(format!("no paper equity snapshot for {date}")))
}

fn raw_bar(
    store: &Store,
    cfg: &Config,
    symbol: &Symbol,
    date: NaiveDate,
) -> Result<pt_core::types::Bar, ShadowError> {
    store
        .bars_asof(symbol, Adjustment::Raw, &cfg.data.feed, date, date, None)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            EngineError::MissingBar {
                symbol: symbol.to_string(),
                date,
            }
            .into()
        })
}

fn raw_closes(
    store: &Store,
    cfg: &Config,
    book: &SimBook,
    date: NaiveDate,
) -> Result<BTreeMap<Symbol, Price>, ShadowError> {
    let mut closes = BTreeMap::new();
    for symbol in book.positions.keys() {
        closes.insert(symbol.clone(), raw_bar(store, cfg, symbol, date)?.close);
    }
    Ok(closes)
}

fn simple_return(
    start: Decimal,
    end: Decimal,
    which: &str,
    prev: NaiveDate,
) -> Result<Decimal, ShadowError> {
    if start.is_zero() {
        return Err(ShadowError::Invalid(format!(
            "{which} equity at the close of {prev} is zero"
        )));
    }
    Ok(end / start - Decimal::ONE)
}

fn to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}
