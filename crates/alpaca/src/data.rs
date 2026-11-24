//! Daily bars from the Alpaca market data host.

use chrono::NaiveDate;
use rust_decimal::Decimal;

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
    // Parsing the body into bars is a later step.
    let _body = http.get_data(&query)?;
    Ok(Vec::new())
}
