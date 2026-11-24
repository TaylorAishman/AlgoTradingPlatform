//! Mapping of Alpaca order statuses to internal order states.

use pt_core::order_state::OrderState;

use crate::error::LiveError;

/// Map an Alpaca order status and filled quantity to an internal [`OrderState`].
pub fn map_status(alpaca_status: &str, filled_qty: u64) -> Result<OrderState, LiveError> {
    match alpaca_status {
        "new"
        | "accepted"
        | "pending_new"
        | "accepted_for_bidding"
        | "held"
        | "done_for_day"
        | "calculated"
        | "stopped"
        | "suspended"
        | "pending_cancel" => Ok(if filled_qty > 0 {
            OrderState::PartiallyFilled
        } else {
            OrderState::Submitted
        }),
        "partially_filled" => Ok(OrderState::PartiallyFilled),
        "filled" => Ok(OrderState::Filled),
        "canceled" => Ok(OrderState::Canceled),
        "expired" => Ok(OrderState::Expired),
        "rejected" => Ok(OrderState::Rejected),
        "replaced" | "pending_replace" => {
            Err(LiveError::UnexpectedStatus(alpaca_status.to_string()))
        }
        other => Err(LiveError::UnknownStatus(other.to_string())),
    }
}
