//! CSV import of bars and calendar data.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;

use crate::error::DataError;

const BAR_COLUMNS: [&str; 7] = ["symbol", "date", "open", "high", "low", "close", "volume"];

fn csv_err(line: usize, message: impl Into<String>) -> DataError {
    DataError::Csv {
        line,
        message: message.into(),
    }
}

fn price(field: &str, line: usize, name: &str) -> Result<Price, DataError> {
    let d = Decimal::from_str(field.trim())
        .map_err(|e| csv_err(line, format!("bad {name} {field:?}: {e}")))?;
    Price::new(d).map_err(|e| csv_err(line, format!("bad {name} {field:?}: {e}")))
}

/// Imports daily bars from CSV text with a header row naming the columns
/// `symbol,date,open,high,low,close,volume` (any order). The bars are stored
/// under the feed `csv:<source>` with the given adjustment. Returns the number
/// of rows read.
pub fn import_bars_csv(
    store: &Store,
    text: &str,
    adjustment: Adjustment,
    source: &str,
    fetched_at: DateTime<Utc>,
) -> Result<usize, DataError> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = reader.headers().map_err(|e| csv_err(1, e.to_string()))?;
    let index: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.to_lowercase(), i))
        .collect();
    let mut cols = [0usize; 7];
    for (slot, name) in cols.iter_mut().zip(BAR_COLUMNS) {
        *slot = *index
            .get(name)
            .ok_or_else(|| csv_err(1, format!("missing column {name}")))?;
    }

    let feed = format!("csv:{source}");
    let mut rows = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let line = i + 2;
        let record = record.map_err(|e| csv_err(line, e.to_string()))?;
        let get = |c: usize| -> Result<&str, DataError> {
            record
                .get(cols[c])
                .ok_or_else(|| csv_err(line, format!("missing field {}", BAR_COLUMNS[c])))
        };
        let symbol = Symbol::new(get(0)?).map_err(|e| csv_err(line, e.to_string()))?;
        let date = NaiveDate::parse_from_str(get(1)?, "%Y-%m-%d")
            .map_err(|e| csv_err(line, format!("bad date: {e}")))?;
        let volume = get(6)?
            .parse::<u64>()
            .map_err(|e| csv_err(line, format!("bad volume: {e}")))?;
        rows.push(BarRow {
            bar: Bar {
                symbol,
                date,
                open: price(get(2)?, line, "open")?,
                high: price(get(3)?, line, "high")?,
                low: price(get(4)?, line, "low")?,
                close: price(get(5)?, line, "close")?,
                volume,
            },
            adjustment,
            feed: feed.clone(),
        });
    }
    store.upsert_bars(&rows, fetched_at)?;
    Ok(rows.len())
}

/// Imports trading session dates from CSV text with a header row containing a
/// `date` column. Nothing is stored if any row is malformed. Returns the
/// number of dates read.
pub fn import_calendar_csv(store: &Store, text: &str) -> Result<usize, DataError> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = reader.headers().map_err(|e| csv_err(1, e.to_string()))?;
    let col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("date"))
        .ok_or_else(|| csv_err(1, "missing column date"))?;

    let mut dates = Vec::new();
    for (i, record) in reader.records().enumerate() {
        let line = i + 2;
        let record = record.map_err(|e| csv_err(line, e.to_string()))?;
        let field = record
            .get(col)
            .ok_or_else(|| csv_err(line, "missing field date"))?;
        let date = NaiveDate::parse_from_str(field, "%Y-%m-%d")
            .map_err(|e| csv_err(line, format!("bad date: {e}")))?;
        dates.push(date);
    }
    store.put_sessions(&dates)?;
    Ok(dates.len())
}
