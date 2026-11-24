//! Order submission. Sending an order needs an `OrderPermit` (architecture D-17).

use pt_gate::gate::{OrderPermit, OrderSummary};

use crate::error::AlpacaError;
use crate::http::HttpClient;
use crate::orders_read::{parse_order, WireOrder};

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
