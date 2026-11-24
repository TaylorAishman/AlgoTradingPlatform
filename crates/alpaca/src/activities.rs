//! Account activities: fills.

use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::error::AlpacaError;
use crate::http::HttpClient;
use crate::orders_read::percent_encode;

/// A fill activity as Alpaca reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireFill {
    pub id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: u64,
    pub price: Decimal,
    pub transaction_time: DateTime<Utc>,
}

/// GET /v2/account/activities/FILL after `after`, oldest first.
/// Full pages of 100 are followed by `page_token` set to the last id.
pub fn fill_activities(
    http: &HttpClient,
    after: DateTime<Utc>,
) -> Result<Vec<WireFill>, AlpacaError> {
    const PAGE: usize = 100;
    let stamp = after.to_rfc3339_opts(SecondsFormat::Secs, true);
    let base = format!("/v2/account/activities/FILL?direction=asc&page_size={PAGE}&after={stamp}");
    let mut out: Vec<WireFill> = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let path = match &token {
            Some(t) => format!("{base}&page_token={}", percent_encode(t)),
            None => base.clone(),
        };
        let body = http.get_trading(&path)?;
        let items: Vec<serde_json::Value> =
            serde_json::from_str(&body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
        let page = items
            .iter()
            .map(parse_fill)
            .collect::<Result<Vec<_>, _>>()?;
        let full = page.len() >= PAGE;
        if let Some(last) = page.last() {
            token = Some(last.id.clone());
        }
        out.extend(page);
        if !full {
            return Ok(out);
        }
    }
}

fn parse_fill(v: &serde_json::Value) -> Result<WireFill, AlpacaError> {
    let text = |k: &str| -> Result<String, AlpacaError> {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .ok_or_else(|| AlpacaError::Parse(format!("fill field {k} missing")))
    };
    let qty = Decimal::from_str(&text("qty")?)
        .map_err(|e| AlpacaError::Parse(format!("fill field qty: {e}")))?;
    if qty.fract() != Decimal::ZERO {
        return Err(AlpacaError::Parse(
            "fill field qty is not whole".to_string(),
        ));
    }
    let qty = qty
        .to_u64()
        .ok_or_else(|| AlpacaError::Parse("fill field qty out of range".to_string()))?;
    let price = Decimal::from_str(&text("price")?)
        .map_err(|e| AlpacaError::Parse(format!("fill field price: {e}")))?;
    let transaction_time = DateTime::parse_from_rfc3339(&text("transaction_time")?)
        .map_err(|e| AlpacaError::Parse(format!("transaction_time: {e}")))?
        .with_timezone(&Utc);
    Ok(WireFill {
        id: text("id")?,
        order_id: text("order_id")?,
        symbol: text("symbol")?,
        side: text("side")?,
        qty,
        price,
        transaction_time,
    })
}
