//! Idempotent submission of planned orders (architecture D-17, D-20).
//!
//! Every order is already stored as `Intent` before anything is sent. Each one
//! is authorised by the gate, looked up at the broker by its client order id,
//! posted with the permit and then marked `Submitted` with the broker's id.

use pt_alpaca::orders_read::order_by_client_id;
use pt_alpaca::orders_write::{submit_order, NewOrder};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::{AuthContext, OrderSummary};
use pt_store::trading::OrderRow;

use crate::context::LiveContext;
use crate::error::LiveError;

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
/// broker's order id. A gate refusal stops the call before anything is sent.
pub fn submit_planned(ctx: &LiveContext, auth: &AuthContext) -> Result<SubmitReport, LiveError> {
    let mut report = SubmitReport::default();
    for row in ctx.store.orders_in_states(&[OrderState::Intent])? {
        let order = new_order(&row);
        let permit = ctx.gate.authorize(&summary(&order), auth)?;
        if order_by_client_id(ctx.http, &order.client_order_id)?.is_some() {
            // The broker already holds it: never send it twice (D-20).
            continue;
        }
        let wire = submit_order(ctx.http, permit, &order)?;
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
