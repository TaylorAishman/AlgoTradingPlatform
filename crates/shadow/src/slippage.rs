//! Slippage of a fill against a reference price.

use crate::error::ShadowError;
use pt_config::config::Config;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Side};
use pt_store::db::Store;
use pt_store::shadow::SlippageRow;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashSet;

/// Slippage in basis points, positive when the fill is worse than the reference.
///
/// A buy is worse when filled above the reference, a sell when filled below it.
/// A zero reference price gives zero.
pub fn slippage_bps(side: Side, fill: Decimal, reference: Decimal) -> f64 {
    if reference.is_zero() {
        return 0.0;
    }
    let diff = match side {
        Side::Buy => fill - reference,
        Side::Sell => reference - fill,
    };
    (diff / reference * Decimal::from(10_000))
        .to_f64()
        .unwrap_or(0.0)
}

/// Record slippage for every order with fills that has no slippage row yet.
///
/// The reference is the raw open of the order's trade date and the fill price
/// is weighted by quantity. Dry-run orders and orders without fills are
/// skipped. Returns the number of rows recorded.
pub fn record_slippage(store: &Store, cfg: &Config) -> Result<usize, ShadowError> {
    use OrderState::*;
    let done: HashSet<String> = store
        .slippage_rows()?
        .into_iter()
        .map(|r| r.client_order_id)
        .collect();
    let fills = store.fills()?;
    let orders = store.orders_in_states(&[
        Submitted,
        PartiallyFilled,
        Filled,
        Canceled,
        Expired,
        Abandoned,
    ])?;
    let mut recorded = 0;
    for order in orders {
        if done.contains(&order.client_order_id) {
            continue;
        }
        let Some(broker_id) = order.broker_order_id.as_deref() else {
            continue;
        };
        let mut qty: u64 = 0;
        let mut notional = Decimal::ZERO;
        for f in fills.iter().filter(|f| f.broker_order_id == broker_id) {
            qty += f.qty;
            notional += f.price * Decimal::from(f.qty);
        }
        if qty == 0 {
            continue;
        }
        let avg_fill_price = (notional / Decimal::from(qty)).normalize();
        let bars = store.bars_asof(
            &order.symbol,
            Adjustment::Raw,
            &cfg.data.feed,
            order.trade_date,
            order.trade_date,
            None,
        )?;
        let reference_price = bars
            .first()
            .ok_or_else(|| {
                ShadowError::NotFound(format!(
                    "no raw bar for {} on {}",
                    order.symbol.as_str(),
                    order.trade_date
                ))
            })?
            .open
            .value();
        let qty = u32::try_from(qty)
            .map_err(|_| ShadowError::Invalid(format!("fill quantity {qty} is too large")))?;
        store.put_slippage(&SlippageRow {
            client_order_id: order.client_order_id,
            reference_price,
            avg_fill_price,
            qty,
            bps: slippage_bps(order.side, avg_fill_price, reference_price),
        })?;
        recorded += 1;
    }
    Ok(recorded)
}
