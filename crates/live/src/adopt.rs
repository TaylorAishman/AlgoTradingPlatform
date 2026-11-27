//! Adopting broker positions and orders that the local store does not yet know about.

use pt_alpaca::orders_read::order_by_client_id;
use pt_core::order_state::OrderState;

use crate::context::LiveContext;
use crate::error::LiveError;

/// Adopt a stored `Rejected` order the broker still holds under its client id.
///
/// Returns `true` when the row moved to `Submitted` with the broker's id. It
/// makes no request unless the row is `Rejected`, never POSTs, and leaves the
/// row alone when the broker does not know the order or holds it as rejected.
pub fn adopt_if_held(ctx: &LiveContext, client_order_id: &str) -> Result<bool, LiveError> {
    let Some(row) = ctx.store.order(client_order_id)? else {
        return Ok(false);
    };
    if row.state != OrderState::Rejected {
        return Ok(false);
    }
    let Some(held) = order_by_client_id(ctx.http, client_order_id)? else {
        return Ok(false);
    };
    if held.status == "rejected" {
        return Ok(false);
    }
    ctx.store.set_order_state(
        client_order_id,
        OrderState::Submitted,
        0,
        Some(&held.id),
        ctx.clock.now_utc(),
    )?;
    Ok(true)
}
