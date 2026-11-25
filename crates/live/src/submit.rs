//! Idempotent submission of planned orders (architecture D-17, D-20).
//!
//! Every order is already stored as `Intent` before anything is sent. Each one
//! is authorised by the gate, looked up at the broker by its client order id,
//! posted with the permit and then marked `Submitted` with the broker's id.

use chrono::NaiveDate;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::orders_read::order_by_client_id;
use pt_alpaca::orders_write::{submit_order, NewOrder};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::{AuthContext, OrderSummary};
use pt_ops::alert::Level;
use pt_store::trading::OrderRow;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::snapshot::expected_last_session;

/// What one call of [`submit_planned`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubmitReport {
    pub submitted: usize,
    pub adopted: usize,
    pub rejected: usize,
    pub abandoned: usize,
}

/// Sends every stored `Intent` order, in plan (insertion) order.
///
/// For each order: the gate authorises it, the broker is asked whether it
/// already holds that client order id, and only if it does not is the order
/// posted with the permit. An accepted order is marked `Submitted` with the
/// broker's order id. An intent whose session has passed is abandoned and
/// alerted instead of sent. A broker rejection marks that order `Rejected`, alerts and carries on. A gate refusal stops the call before anything is sent.
pub fn submit_planned(ctx: &LiveContext, auth: &AuthContext) -> Result<SubmitReport, LiveError> {
    let mut report = SubmitReport::default();
    // Fetched once, after the first gate check, so no network call precedes it.
    let mut oldest_live: Option<NaiveDate> = None;
    for row in ctx.store.orders_in_states(&[OrderState::Intent])? {
        let order = new_order(&row);
        let permit = ctx.gate.authorize(&summary(&order), auth)?;
        if oldest_live.is_none() {
            let clock = pt_alpaca::reference::clock(ctx.http)?;
            let calendar = ctx.store.calendar()?;
            oldest_live = Some(expected_last_session(&clock, &calendar).unwrap_or(NaiveDate::MIN));
        }
        if oldest_live.is_some_and(|oldest| row.trade_date < oldest) {
            // Its session has passed: never send it (R-285).
            let now = ctx.clock.now_utc();
            ctx.store
                .set_order_state(&row.client_order_id, OrderState::Abandoned, 0, None, now)?;
            ctx.alerter.alert(
                ctx.store,
                Level::Warn,
                "order_abandoned",
                &format!(
                    "order {} abandoned: its session {} has passed",
                    row.client_order_id, row.trade_date
                ),
                None,
                now,
            );
            report.abandoned += 1;
            continue;
        }
        if let Some(held) = order_by_client_id(ctx.http, &order.client_order_id)? {
            // The broker already holds it: adopt it, never send it twice (D-20).
            ctx.store.set_order_state(
                &row.client_order_id,
                OrderState::Submitted,
                0,
                Some(&held.id),
                ctx.clock.now_utc(),
            )?;
            report.adopted += 1;
            continue;
        }
        let wire = match submit_order(ctx.http, permit, &order) {
            Ok(wire) => wire,
            Err(AlpacaError::Rejected { status, message }) => {
                // The broker refused this order: record it and carry on (R-286).
                let now = ctx.clock.now_utc();
                ctx.store.set_order_state(
                    &row.client_order_id,
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
                        "order {} for {} rejected by the broker (HTTP {status}): {message}",
                        row.client_order_id,
                        row.symbol.as_str()
                    ),
                    None,
                    now,
                );
                report.rejected += 1;
                continue;
            }
            Err(other) => return Err(other.into()),
        };
        ctx.store.set_order_state(
            &row.client_order_id,
            OrderState::Submitted,
            0,
            Some(&wire.id),
            ctx.clock.now_utc(),
        )?;
        report.submitted += 1;
    }
    Ok(report)
}

fn new_order(row: &OrderRow) -> NewOrder {
    NewOrder {
        client_order_id: row.client_order_id.clone(),
        symbol: row.symbol.as_str().to_string(),
        side: side_text(row.side).to_string(),
        qty: row.qty,
    }
}

fn summary(order: &NewOrder) -> OrderSummary {
    OrderSummary {
        client_order_id: order.client_order_id.clone(),
        symbol: order.symbol.clone(),
        side: order.side.clone(),
        qty: order.qty,
    }
}

fn side_text(side: Side) -> &'static str {
    match side {
        Side::Buy => "buy",
        Side::Sell => "sell",
    }
}
