//! Bar ingest from the Alpaca market data host into the store.

use chrono::{DateTime, NaiveDate, Utc};
use pt_alpaca::data::{daily_bars, WireBar};
use pt_alpaca::http::HttpClient;
use pt_config::config::Config;
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::market::{BarRow, UpsertStats};

use crate::error::DataError;

/// What one ingest stored for each series, and whether every symbol has a
/// raw and an adjusted bar for the `through` session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestReport {
    pub raw: UpsertStats,
    pub adjusted: UpsertStats,
    pub complete: bool,
}

/// R-120: fetches raw and adjusted (`all`) daily bars for every universe
/// symbol and the risk-free symbol from `cfg.data.start` through `through`,
/// one request per series, and stores them under `cfg.data.feed` as fetched
/// at `fetched_at`. Both series are fetched before anything is stored.
pub fn ingest(
    store: &Store,
    http: &HttpClient,
    cfg: &Config,
    through: NaiveDate,
    fetched_at: DateTime<Utc>,
) -> Result<IngestReport, DataError> {
    let symbols = symbols(cfg);
    let feed = cfg.data.feed.as_str();
    let start = cfg.data.start;

    let raw_rows = fetch(http, &symbols, start, through, Adjustment::Raw, feed)?;
    let adjusted_rows = fetch(http, &symbols, start, through, Adjustment::All, feed)?;

    let complete =
        covers(&symbols, &raw_rows, through) && covers(&symbols, &adjusted_rows, through);
    let raw = store.upsert_bars(&raw_rows, fetched_at)?;
    let adjusted = store.upsert_bars(&adjusted_rows, fetched_at)?;
    Ok(IngestReport {
        raw,
        adjusted,
        complete,
    })
}

/// The universe symbols followed by the risk-free symbol, without repeats.
fn symbols(cfg: &Config) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let names = cfg
        .universe
        .iter()
        .map(|i| i.symbol.as_str())
        .chain(std::iter::once(cfg.strategy.risk_free_symbol.as_str()));
    for name in names {
        if !out.iter().any(|s| s == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// Fetches one series and converts it to store rows.
fn fetch(
    http: &HttpClient,
    symbols: &[String],
    start: NaiveDate,
    end: NaiveDate,
    adjustment: Adjustment,
    feed: &str,
) -> Result<Vec<BarRow>, DataError> {
    let wire = daily_bars(
        http,
        symbols,
        start,
        end,
        adjustment_param(adjustment),
        feed,
    )?;
    wire.iter()
        .map(|w| {
            Ok(BarRow {
                bar: to_bar(w)?,
                adjustment,
                feed: feed.to_string(),
            })
        })
        .collect()
}

/// The `adjustment` query value for a series.
fn adjustment_param(adjustment: Adjustment) -> &'static str {
    match adjustment {
        Adjustment::Raw => "raw",
        Adjustment::All => "all",
    }
}

fn to_bar(w: &WireBar) -> Result<Bar, DataError> {
    let bad = |message: String| DataError::BadBar {
        symbol: w.symbol.clone(),
        date: w.date,
        message,
    };
    let symbol = Symbol::new(&w.symbol).map_err(|e| bad(format!("bad symbol: {e}")))?;
    let price = |d, name: &str| Price::new(d).map_err(|e| bad(format!("bad {name}: {e}")));
    Ok(Bar {
        symbol,
        date: w.date,
        open: price(w.open, "open")?,
        high: price(w.high, "high")?,
        low: price(w.low, "low")?,
        close: price(w.close, "close")?,
        volume: w.volume,
    })
}

/// True when every symbol has a row dated `through`.
fn covers(symbols: &[String], rows: &[BarRow], through: NaiveDate) -> bool {
    symbols.iter().all(|s| {
        rows.iter()
            .any(|r| r.bar.date == through && r.bar.symbol.as_str() == s)
    })
}
