//! Settlement: bring every open order's state up to date from the broker and
//! record its fills.
//!
//! Fills come from the FILL activities, keyed by activity id (D-19), so
//! reading the same activity again records nothing more.

use std::collections::{HashMap, HashSet};

use chrono::{NaiveTime, TimeZone, Utc};
use pt_alpaca::activities::{fill_activities, WireFill};
use pt_alpaca::orders_read::order_by_client_id;
use pt_core::order_state::OrderState;
use pt_store::trading::{Book, FillRow, LedgerEntry, OrderRow};
use rust_decimal::Decimal;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::status::map_status;

/// What one call of [`settle`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettleReport {
    pub orders_updated: usize,
    pub fills_recorded: usize,
}

/// Asks the broker about every open (`Submitted` or `PartiallyFilled`) order,
/// records each of its fills not yet stored together with a Paper ledger
/// entry, then stores its new state and filled quantity when either has
/// changed. An order the broker does not know is left as it is.
///
/// The stored state never runs ahead of the stored fills: the filled quantity
/// written is at most the sum of the order's stored fills, and the order
/// leaves the open states only when those fills cover what the broker
/// reports filled (the whole order, for `filled`). When the broker's order
/// record is ahead of its activity feed, the order stays `Submitted` or
/// `PartiallyFilled` at the stored quantity, and a later settle picks up the
/// missing fills and the final state.
pub fn settle(ctx: &LiveContext) -> Result<SettleReport, LiveError> {
    let mut report = SettleReport::default();
    let open = [OrderState::Submitted, OrderState::PartiallyFilled];
    let rows = ctx.store.orders_in_states(&open)?;
    let Some(earliest) = rows.iter().map(|r| r.trade_date).min() else {
        return Ok(report);
    };
    let after = Utc.from_utc_datetime(&earliest.and_time(NaiveTime::MIN));
    let activities = fill_activities(ctx.http, after)?;
    let mut ledgered: HashSet<String> = ctx
        .store
        .ledger(Book::Paper)?
        .into_iter()
        .filter(|e| e.kind == "fill")
        .filter_map(|e| e.ref_id)
        .collect();
    let mut stored: HashMap<String, u64> = HashMap::new();
    for f in ctx.store.fills()? {
        *stored.entry(f.broker_order_id).or_default() += f.qty;
    }

    for row in rows {
        let Some(wire) = order_by_client_id(ctx.http, &row.client_order_id)? else {
            continue;
        };
        for fill in activities.iter().filter(|f| f.order_id == wire.id) {
            if record_fill(ctx, &row, fill, &mut ledgered)? {
                report.fills_recorded += 1;
                *stored.entry(fill.order_id.clone()).or_default() += fill.qty;
            }
        }
        let broker_state = map_status(&wire.status, wire.filled_qty)?;
        let stored_qty = stored.get(&wire.id).copied().unwrap_or(0);
        let (state, filled_qty) = supported(broker_state, wire.filled_qty, row.qty, stored_qty);
        if state == row.state && filled_qty == row.filled_qty {
            continue;
        }
        if matches!(state, OrderState::Expired | OrderState::Canceled)
            && filled_qty > 0
            && filled_qty < row.qty
        {
            // Recorded before the state change: a crash between the two can
            // repeat the alert on the next settle, never lose it.
            ctx.store.record_alert(
                "warn",
                "incomplete_fill",
                &format!(
                    "{} order {} ended {state:?} with {} of {} filled",
                    row.symbol, row.client_order_id, filled_qty, row.qty
                ),
                false,
                ctx.clock.now_utc(),
            )?;
        }
        ctx.store.set_order_state(
            &row.client_order_id,
            state,
            filled_qty,
            Some(&wire.id),
            ctx.clock.now_utc(),
        )?;
        report.orders_updated += 1;
    }
    Ok(report)
}

/// The state and filled quantity the stored fills support. The broker claims
/// `wire_filled` shares (the whole `order_qty` when its state is `Filled`);
/// while fewer than that are stored, the order stays open at the stored
/// quantity so the next settle retries.
fn supported(
    broker_state: OrderState,
    wire_filled: u64,
    order_qty: u64,
    stored_qty: u64,
) -> (OrderState, u64) {
    let claimed = if broker_state == OrderState::Filled {
        wire_filled.max(order_qty)
    } else {
        wire_filled
    };
    if stored_qty >= claimed {
        (broker_state, claimed)
    } else if stored_qty > 0 {
        (OrderState::PartiallyFilled, stored_qty)
    } else {
        (OrderState::Submitted, 0)
    }
}

/// Stores one fill of `row` and its Paper ledger entry, dated by the fill's
/// UTC date. Returns true when the fill row is new. A fill already stored
/// whose ledger entry is missing gets the entry, so the two stay paired.
fn record_fill(
    ctx: &LiveContext,
    row: &OrderRow,
    fill: &WireFill,
    ledgered: &mut HashSet<String>,
) -> Result<bool, LiveError> {
    let inserted = ctx.store.insert_fill(&FillRow {
        activity_id: fill.id.clone(),
        broker_order_id: fill.order_id.clone(),
        symbol: row.symbol.clone(),
        side: row.side,
        qty: fill.qty,
        price: fill.price,
        at: fill.transaction_time,
    })?;
    if !ledgered.contains(&fill.id) {
        let qty = i64::try_from(fill.qty)
            .map_err(|_| LiveError::Invalid(format!("fill quantity {} too large", fill.qty)))?
            * row.side.sign();
        ctx.store.append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: fill.transaction_time.date_naive(),
            kind: "fill".to_string(),
            symbol: Some(row.symbol.clone()),
            qty,
            price: Some(fill.price),
            cash_delta: -(Decimal::from(qty) * fill.price),
            ref_id: Some(fill.id.clone()),
        })?;
        ledgered.insert(fill.id.clone());
    }
    Ok(inserted)
}
