//! Reference data endpoints: the market clock and the trading calendar.

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::error::AlpacaError;
use crate::http::HttpClient;

/// The market clock as Alpaca reports it; instants keep their offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireClock {
    pub timestamp: DateTime<FixedOffset>,
    pub is_open: bool,
    pub next_open: DateTime<FixedOffset>,
    pub next_close: DateTime<FixedOffset>,
}

/// GET /v2/clock on the trading base URL.
pub fn clock(http: &HttpClient) -> Result<WireClock, AlpacaError> {
    let body = http.get_trading("/v2/clock")?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
    let instant = |k: &str| -> Result<DateTime<FixedOffset>, AlpacaError> {
        let s = v
            .get(k)
            .and_then(|x| x.as_str())
            .ok_or_else(|| AlpacaError::Parse(format!("clock field {k} missing")))?;
        DateTime::parse_from_rfc3339(s).map_err(|e| AlpacaError::Parse(format!("{k}: {e}")))
    };
    Ok(WireClock {
        timestamp: instant("timestamp")?,
        is_open: v
            .get("is_open")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| AlpacaError::Parse("clock field is_open missing".to_string()))?,
        next_open: instant("next_open")?,
        next_close: instant("next_close")?,
    })
}

/// GET /v2/calendar?start=&end= on the trading base URL; the session dates.
pub fn calendar(
    http: &HttpClient,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<NaiveDate>, AlpacaError> {
    let path = format!(
        "/v2/calendar?start={}&end={}",
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d")
    );
    let body = http.get_trading(&path)?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
    let items = v
        .as_array()
        .ok_or_else(|| AlpacaError::Parse("calendar body is not an array".to_string()))?;
    items
        .iter()
        .map(|item| {
            let s = item
                .get("date")
                .and_then(|x| x.as_str())
                .ok_or_else(|| AlpacaError::Parse("calendar field date missing".to_string()))?;
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|e| AlpacaError::Parse(format!("date: {e}")))
        })
        .collect()
}
