//! Order submission and cancellation. Sending an order needs an `OrderPermit`
//! (architecture D-17); cancelling one does not (architecture 5.5).

use pt_gate::gate::{OrderPermit, OrderSummary};

use crate::error::AlpacaError;
use crate::http::HttpClient;
use crate::orders_read::{parse_order, percent_encode, WireOrder};

/// A market day order to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOrder {
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: u64,
}

/// Sends `order` as a market day order and returns the order Alpaca reports.
///
/// The permit is spent either way. If it was not issued for this exact order
/// (client order id, symbol, side and quantity), `PermitMismatch` is returned
/// and nothing is sent: no limiter slot is taken (R-100).
pub fn submit_order(
    http: &HttpClient,
    permit: OrderPermit,
    order: &NewOrder,
) -> Result<WireOrder, AlpacaError> {
    let summary = OrderSummary {
        client_order_id: order.client_order_id.clone(),
        symbol: order.symbol.clone(),
        side: order.side.clone(),
        qty: order.qty,
    };
    if !permit.matches(&summary) {
        return Err(AlpacaError::PermitMismatch);
    }
    drop(permit);
    let body = serde_json::json!({
        "symbol": order.symbol,
        "qty": order.qty.to_string(),
        "side": order.side,
        "type": "market",
        "time_in_force": "day",
        "client_order_id": order.client_order_id,
    })
    .to_string();
    let reply = http.post_trading("/v2/orders", &body)?;
    parse_order(&reply)
}

/// Cancels the order with broker id `broker_order_id` (R-102).
///
/// Sends DELETE `/v2/orders/{broker_order_id}`. Any 2xx is `Ok(())` and its
/// body is ignored. No permit is taken: whether a cancel is allowed is the
/// caller's check, `Gate::allows(Capability::CancelOrder)`.
pub fn cancel_order(http: &HttpClient, broker_order_id: &str) -> Result<(), AlpacaError> {
    http.delete_trading(&format!("/v2/orders/{}", percent_encode(broker_order_id)))?;
    Ok(())
}
