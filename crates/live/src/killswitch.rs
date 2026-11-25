//! The kill switch: cancel the open orders at the broker.

use pt_alpaca::error::AlpacaError;
use pt_alpaca::orders_read::order_by_client_id;
use pt_alpaca::orders_write::cancel_order;
use pt_core::order_state::OrderState;
use pt_gate::gate::Capability;

use crate::context::LiveContext;
use crate::error::LiveError;

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
    ctx.gate.allows(Capability::CancelOrder)?;
    let open = [OrderState::Submitted, OrderState::PartiallyFilled];
    let mut cancelled = 0;
    for row in ctx.store.orders_in_states(&open)? {
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
