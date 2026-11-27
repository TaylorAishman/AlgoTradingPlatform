use pt_alpaca::reference::WireClock;
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::ids::{first_order_id, first_order_retry_id};
use crate::snapshot::trade_session;

/// Plans the owner's approved first order as an `Intent` order, but only on
/// the approval's day. Returns whether an order is planned (already present
/// counts). Writes no decision and sends nothing.
pub fn first_order(ctx: &LiveContext, wire_clock: &WireClock) -> Result<bool, LiveError> {
    let Some(approval) = ctx.gate.first_order() else {
        return Ok(false);
    };
    if approval.valid_on != trade_session(wire_clock) {
        return Ok(false);
    }
    let side = match approval.side.as_str() {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        other => {
            return Err(LiveError::Invalid(format!(
                "first order side \"{other}\" is not buy or sell"
            )))
        }
    };
    let symbol = Symbol::new(&approval.symbol)
        .map_err(|e| LiveError::Invalid(format!("symbol \"{}\": {e}", approval.symbol)))?;
    let res = ctx.store.insert_order(&OrderRow {
        client_order_id: first_order_id(approval.valid_on, &approval.symbol),
        decision_id: None,
        symbol,
        side,
        qty: approval.qty,
        leg: 0,
        state: OrderState::Intent,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::ZERO,
        trade_date: approval.valid_on,
    });
    match res {
        Ok(()) | Err(StoreError::Duplicate(_)) => Ok(true),
        Err(e) => Err(e.into()),
    }
}

/// Waits between a rejected first order and each retry (R-495), in seconds.
pub const RETRY_WAITS_SECS: [u64; 5] = [1, 2, 4, 8, 16];

/// Plans the `attempt`th retry (counting from 1) of the approved first order
/// as a new `Intent` order under a new client order id (R-494). Returns
/// whether an order is planned; false when there is no approval for today.
pub fn plan_first_order_retry(
    ctx: &LiveContext,
    wire_clock: &WireClock,
    attempt: u32,
) -> Result<bool, LiveError> {
    let Some(approval) = ctx.gate.first_order() else {
        return Ok(false);
    };
    if approval.valid_on != trade_session(wire_clock) {
        return Ok(false);
    }
    let side = match approval.side.as_str() {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        other => {
            return Err(LiveError::Invalid(format!(
                "first order side \"{other}\" is not buy or sell"
            )))
        }
    };
    let symbol = Symbol::new(&approval.symbol)
        .map_err(|e| LiveError::Invalid(format!("symbol \"{}\": {e}", approval.symbol)))?;
    let res = ctx.store.insert_order(&OrderRow {
        client_order_id: first_order_retry_id(approval.valid_on, &approval.symbol, attempt),
        decision_id: None,
        symbol,
        side,
        qty: approval.qty,
        leg: 0,
        state: OrderState::Intent,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::ZERO,
        trade_date: approval.valid_on,
    });
    match res {
        Ok(()) | Err(StoreError::Duplicate(_)) => Ok(true),
        Err(e) => Err(e.into()),
    }
}

/// Whether the broker has accepted a first order (R-493): any stored
/// first-order row, retries included, in a state a broker acceptance leads to.
/// An `Intent` or `Rejected` row does not count.
pub fn first_order_accepted(store: &Store) -> Result<bool, LiveError> {
    let accepted = store.orders_in_states(&[
        OrderState::Submitted,
        OrderState::PartiallyFilled,
        OrderState::Filled,
        OrderState::Canceled,
        OrderState::Expired,
    ])?;
    Ok(accepted
        .iter()
        .any(|o| o.client_order_id.starts_with("pt-first-")))
}
