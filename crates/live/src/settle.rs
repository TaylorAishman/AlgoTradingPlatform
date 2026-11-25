//! Settlement: bring every open order's state up to date from the broker.
//!
//! Fills and ledger entries are recorded elsewhere; this only updates states.

use pt_alpaca::orders_read::order_by_client_id;
use pt_core::order_state::OrderState;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::status::map_status;

/// What one call of [`settle`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettleReport {
    pub orders_updated: usize,
}

/// Asks the broker about every open (`Submitted` or `PartiallyFilled`) order
/// and stores its new state and filled quantity when either has changed.
/// An order the broker does not know is left as it is.
pub fn settle(ctx: &LiveContext) -> Result<SettleReport, LiveError> {
    let mut report = SettleReport::default();
    let open = [OrderState::Submitted, OrderState::PartiallyFilled];
    for row in ctx.store.orders_in_states(&open)? {
        let Some(wire) = order_by_client_id(ctx.http, &row.client_order_id)? else {
            continue;
        };
        let state = map_status(&wire.status, wire.filled_qty)?;
        if state == row.state && wire.filled_qty == row.filled_qty {
            continue;
        }
        ctx.store.set_order_state(
            &row.client_order_id,
            state,
            wire.filled_qty,
            Some(&wire.id),
            ctx.clock.now_utc(),
        )?;
        report.orders_updated += 1;
    }
    Ok(report)
}
