//! Order lookups.

use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::error::AlpacaError;
use crate::http::HttpClient;

/// An order as Alpaca reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireOrder {
    pub id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: u64,
    pub filled_qty: u64,
    pub status: String,
    pub submitted_at: DateTime<Utc>,
}

/// Percent-encodes everything except RFC 3986 unreserved characters.
pub(crate) fn percent_encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// GET /v2/orders:by_client_order_id. A 404 means no such order: `Ok(None)`.
pub fn order_by_client_id(
    http: &HttpClient,
    client_order_id: &str,
) -> Result<Option<WireOrder>, AlpacaError> {
    let path = format!(
        "/v2/orders:by_client_order_id?client_order_id={}",
        percent_encode(client_order_id)
    );
    match http.get_trading(&path) {
        Ok(body) => parse_order(&body).map(Some),
        Err(AlpacaError::Rejected { status: 404, .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// GET /v2/orders for every status submitted after `after`, oldest first.
/// Pages of 500 are followed by the last order's `submitted_at`.
pub fn orders_since(
    http: &HttpClient,
    after: DateTime<Utc>,
) -> Result<Vec<WireOrder>, AlpacaError> {
    const PAGE: usize = 500;
    let mut out = Vec::new();
    let mut cursor = after;
    loop {
        let stamp = cursor.to_rfc3339_opts(SecondsFormat::AutoSi, true);
        let path = format!("/v2/orders?status=all&direction=asc&limit={PAGE}&after={stamp}");
        let body = http.get_trading(&path)?;
        let items: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
        let page = items
            .iter()
            .map(parse_order_value)
            .collect::<Result<Vec<_>, _>>()?;
        let full = page.len() >= PAGE;
        if let Some(last) = page.last() {
            if full && last.submitted_at <= cursor {
                return Err(AlpacaError::Parse(
                    "orders paging cursor did not advance".to_string(),
                ));
            }
            cursor = last.submitted_at;
        }
        out.extend(page);
        if !full {
            return Ok(out);
        }
    }
}

pub(crate) fn parse_order(body: &str) -> Result<WireOrder, AlpacaError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
    parse_order_value(&v)
}

fn parse_order_value(v: &serde_json::Value) -> Result<WireOrder, AlpacaError> {
    let text = |k: &str| -> Result<String, AlpacaError> {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .ok_or_else(|| AlpacaError::Parse(format!("order field {k} missing")))
    };
    let count = |k: &str| -> Result<u64, AlpacaError> {
        let d = Decimal::from_str(&text(k)?)
            .map_err(|e| AlpacaError::Parse(format!("order field {k}: {e}")))?;
        if d.fract() != Decimal::ZERO {
            return Err(AlpacaError::Parse(format!("order field {k} is not whole")));
        }
        d.to_u64()
            .ok_or_else(|| AlpacaError::Parse(format!("order field {k} out of range")))
    };
    let submitted_at = DateTime::parse_from_rfc3339(&text("submitted_at")?)
        .map_err(|e| AlpacaError::Parse(format!("submitted_at: {e}")))?
        .with_timezone(&Utc);
    Ok(WireOrder {
        id: text("id")?,
        client_order_id: text("client_order_id")?,
        symbol: text("symbol")?,
        side: text("side")?,
        qty: count("qty")?,
        filled_qty: count("filled_qty")?,
        status: text("status")?,
        submitted_at,
    })
}
