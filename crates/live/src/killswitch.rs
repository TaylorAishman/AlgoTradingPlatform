//! The kill switch: cancel the open orders at the broker, or flatten the account.

use chrono::NaiveDate;
use pt_alpaca::account::positions;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::orders_read::order_by_client_id;
use pt_alpaca::orders_write::{cancel_order, submit_order, NewOrder};
use pt_alpaca::reference;
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_gate::gate::{Capability, OrderSummary};
use pt_ops::alert::Level;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::snapshot::trade_session;

/// Cancels every open (`Submitted` or `PartiallyFilled`) order at the broker
/// and returns how many cancels were sent.
///
/// Only the stage is checked: an active halt does not stop this, because the
/// kill switch has to work while halted. Stored order states are left for
/// [`crate::settle::settle`] to update.
///
/// A stored open row can be stale (filled or cancelled at the broker but not
/// yet settled). The broker answers its cancel with 404 or 422; that order is
/// skipped, not counted, and the rest are still cancelled. Any other error
/// stops the call.
pub fn cancel_open(ctx: &LiveContext) -> Result<usize, LiveError> {
    cancel_matching(ctx, false)
}

/// Cancels the open orders, leaving working flatten orders alone when
/// `keep_flatten` is set: they are the closing orders themselves.
fn cancel_matching(ctx: &LiveContext, keep_flatten: bool) -> Result<usize, LiveError> {
    ctx.gate.allows(Capability::CancelOrder)?;
    let open = [OrderState::Submitted, OrderState::PartiallyFilled];
    let mut cancelled = 0;
    for row in ctx.store.orders_in_states(&open)? {
        if keep_flatten && row.client_order_id.starts_with("pt-flatten-") {
            continue;
        }
        let broker_id = match row.broker_order_id {
            Some(id) => id,
            None => match order_by_client_id(ctx.http, &row.client_order_id)? {
                Some(wire) => wire.id,
                None => continue,
            },
        };
        match cancel_order(ctx.http, &broker_id) {
            Ok(()) => cancelled += 1,
            // The stored row is stale: the broker no longer holds it open.
            Err(AlpacaError::Rejected {
                status: 404 | 422, ..
            }) => {}
            Err(other) => return Err(other.into()),
        }
    }
    Ok(cancelled)
}

/// What [`flatten`] did: orders cancelled, orders the broker accepted and
/// orders the broker rejected. Adopted and already-working orders count in none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlattenReport {
    /// Open orders cancelled before closing.
    pub cancelled: usize,
    /// Orders sent and accepted by the broker.
    pub accepted: usize,
    /// Orders the broker rejected.
    pub rejected: usize,
}

/// Closes every position the broker holds with one market day order per
/// symbol, in symbol order, and returns a [`FlattenReport`].
///
/// Works in every stage, with no flat confirmation or allowlist, and an active
/// halt does not stop it. A `flattened` halt is set (unless one is already active) before any order
/// is authorised or sent, so the system stays halted even if the gate refuses
/// an order or sending fails part way. Each order is authorised by the gate's
/// kill-switch rule (`authorize_kill_switch`), stored with client order id `pt-flatten-<yyyymmdd>-<symbol>`
/// so [`crate::settle::settle`] records its fills, and is sent only if the
/// broker does not already hold that id. A stored flatten order carries no
/// decision and a zero reference price: no plan priced it.
///
/// The stored row for that id decides what happens. A `Submitted` or
/// `PartiallyFilled` row is still working at the broker, so nothing is sent.
/// A `Rejected` row whose order the broker nevertheless kept is adopted: the
/// row becomes `Submitted` with the broker's id, nothing is sent and it counts
/// as accepted. A row in any other state but `Intent` (filled, cancelled, ...) is
/// spent, so a retry uses the next free id `<id>-r1`, `<id>-r2`, ... and never
/// reuses a spent one. With no row or an `Intent` row, an order the broker
/// already holds (an earlier flatten today) is adopted: the row is marked
/// `Submitted` with the broker's id and the order is not sent again.
///
/// A broker rejection (HTTP 403 or 422 only) marks that order `Rejected`, records an `order_rejected`
/// alert and carries on with the remaining symbols, so one refused symbol
/// never stops the rest from closing. The report counts the orders sent
/// and accepted by this call and those rejected; working and adopted orders
/// are in neither, and a partial failure is reported only through its alert. A gate
/// refusal or any other error stops the call.
///
/// Once the halt is set, `flatten` cancels every open order except its own working flatten orders ([`cancel_open`]),
/// then settles ([`crate::settle::settle`]) so cancelled orders and their
/// partial fills are stored, and only then reads the positions to close.
pub fn flatten(ctx: &LiveContext) -> Result<FlattenReport, LiveError> {
    if !ctx
        .store
        .active_halts()?
        .iter()
        .any(|h| h.reason == "flattened")
    {
        ctx.store.set_halt(
            "flattened",
            "the owner flattened the account",
            ctx.clock.now_utc(),
        )?;
    }
    let cancelled = cancel_matching(ctx, true)?;
    crate::settle::settle(ctx)?;
    let today = trade_session(&reference::clock(ctx.http)?);
    let mut held = positions(ctx.http)?;
    held.retain(|p| p.qty != 0);
    held.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let mut placed = 0;
    let mut rejected = 0;
    for p in held {
        let base = flatten_order_id(today, &p.symbol);
        let (mut client_order_id, mut retry) = (base.clone(), 0);
        let mut adopted = false;
        let stored = loop {
            let row = ctx.store.order(&client_order_id)?;
            match row.as_ref().map(|r| r.state) {
                None | Some(OrderState::Intent) => break row,
                Some(OrderState::Submitted | OrderState::PartiallyFilled) => break row,
                Some(OrderState::Rejected) => {
                    // The broker may have kept the order it answered with a
                    // reject: adopt it rather than send a second one.
                    if let Some(held) = order_by_client_id(ctx.http, &client_order_id)? {
                        ctx.store.set_order_state(
                            &client_order_id,
                            OrderState::Submitted,
                            0,
                            Some(&held.id),
                            ctx.clock.now_utc(),
                        )?;
                        adopted = true;
                        break row;
                    }
                    retry += 1;
                    client_order_id = format!("{base}-r{retry}");
                }
                Some(_) => {
                    retry += 1;
                    client_order_id = format!("{base}-r{retry}");
                }
            }
        };
        if adopted {
            placed += 1;
            continue;
        }
        if stored
            .as_ref()
            .is_some_and(|r| r.state != OrderState::Intent)
        {
            continue; // still working at the broker
        }
        let side = if p.qty > 0 { Side::Sell } else { Side::Buy };
        let order = NewOrder {
            client_order_id,
            symbol: p.symbol.clone(),
            side: match side {
                Side::Buy => "buy",
                Side::Sell => "sell",
            }
            .to_string(),
            qty: p.qty.unsigned_abs(),
        };
        let permit = ctx.gate.authorize_kill_switch(
            &OrderSummary {
                client_order_id: order.client_order_id.clone(),
                symbol: order.symbol.clone(),
                side: order.side.clone(),
                qty: order.qty,
            },
            p.qty,
        )?;
        if stored.is_none() {
            let symbol = Symbol::new(&p.symbol)
                .map_err(|e| LiveError::Invalid(format!("position symbol: {e}")))?;
            ctx.store.insert_order(&OrderRow {
                client_order_id: order.client_order_id.clone(),
                decision_id: None,
                symbol,
                side,
                qty: order.qty,
                leg: 0,
                state: OrderState::Intent,
                broker_order_id: None,
                filled_qty: 0,
                reference_price: Decimal::ZERO,
                trade_date: today,
            })?;
        }
        if let Some(held) = order_by_client_id(ctx.http, &order.client_order_id)? {
            // Already sent by an earlier flatten today: adopt it, never send it twice.
            ctx.store.set_order_state(
                &order.client_order_id,
                OrderState::Submitted,
                0,
                Some(&held.id),
                ctx.clock.now_utc(),
            )?;
            continue;
        }
        let wire = match submit_order(ctx.http, permit, &order) {
            Ok(wire) => wire,
            Err(AlpacaError::Rejected { status, message }) if matches!(status, 403 | 422) => {
                // The broker refused this one: record it and close the rest.
                // Any other status (401, 429, 5xx) is not a refusal of the order
                // and falls through to stop the call (R-489).
                let now = ctx.clock.now_utc();
                ctx.store.set_order_state(
                    &order.client_order_id,
                    OrderState::Rejected,
                    0,
                    None,
                    now,
                )?;
                ctx.alerter.alert(
                    ctx.store,
                    Level::Warn,
                    "order_rejected",
                    &format!(
                        "flatten order {} for {} rejected by the broker (HTTP {status}): {message}",
                        order.client_order_id, order.symbol
                    ),
                    None,
                    now,
                );
                rejected += 1;
                continue;
            }
            Err(other) => return Err(other.into()),
        };
        ctx.store.set_order_state(
            &order.client_order_id,
            OrderState::Submitted,
            0,
            Some(&wire.id),
            ctx.clock.now_utc(),
        )?;
        placed += 1;
    }
    Ok(FlattenReport {
        cancelled,
        accepted: placed,
        rejected,
    })
}

/// Identifier of a flatten order: `pt-flatten-<yyyymmdd>-<symbol>`.
fn flatten_order_id(date: NaiveDate, symbol: &str) -> String {
    format!("pt-flatten-{}-{symbol}", date.format("%Y%m%d"))
}
