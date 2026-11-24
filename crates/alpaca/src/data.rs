//! Daily bars from the Alpaca market data host.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::error::AlpacaError;
use crate::http::HttpClient;

/// One daily bar as the wire carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireBar {
    pub symbol: String,
    pub date: NaiveDate,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
}

/// Ask the market data host for daily bars of `symbols` between `start` and `end`.
pub fn daily_bars(
    http: &HttpClient,
    symbols: &[String],
    start: NaiveDate,
    end: NaiveDate,
    adjustment: &str,
    feed: &str,
) -> Result<Vec<WireBar>, AlpacaError> {
    let query = format!(
        "/v2/stocks/bars?symbols={}&timeframe=1Day&start={}&end={}&adjustment={}&feed={}&limit=10000&sort=asc",
        symbols.join(","),
        start.format("%Y-%m-%d"),
        end.format("%Y-%m-%d"),
        adjustment,
        feed,
    );
    let body = http.get_data(&query)?;
    parse_page(&body)
}

#[derive(Deserialize)]
struct Page {
    bars: Option<BTreeMap<String, Vec<RawBar>>>,
}

/// Prices are `Decimal` fields of a derived `Deserialize`, which reads a JSON number
/// through the shortest round-trip text of its `f64` (architecture D-11). No arithmetic
/// is done on the float.
#[derive(Deserialize)]
struct RawBar {
    t: String,
    o: Decimal,
    h: Decimal,
    l: Decimal,
    c: Decimal,
    v: u64,
}

fn parse_page(body: &str) -> Result<Vec<WireBar>, AlpacaError> {
    let page: Page = serde_json::from_str(body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
    let mut out = Vec::new();
    for (symbol, bars) in page.bars.unwrap_or_default() {
        for b in bars {
            let utc = DateTime::parse_from_rfc3339(&b.t)
                .map_err(|e| AlpacaError::Parse(format!("bar time {}: {e}", b.t)))?
                .naive_utc();
            // Midnight New York is 05:00Z in winter and 04:00Z in summer (R-091).
            let midnight_ny = [4, 5]
                .iter()
                .any(|h| Some(utc.time()) == NaiveTime::from_hms_opt(*h, 0, 0));
            if !midnight_ny {
                return Err(AlpacaError::UnexpectedBarTime(format!(
                    "{symbol} bar at {}",
                    b.t
                )));
            }
            let date = utc.date();
            out.push(WireBar {
                symbol: symbol.clone(),
                date,
                open: b.o,
                high: b.h,
                low: b.l,
                close: b.c,
                volume: b.v,
            });
        }
    }
    Ok(out)
}
