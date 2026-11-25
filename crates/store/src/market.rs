//! Market data repositories: versioned daily bars.

use chrono::{DateTime, SecondsFormat, Utc};
use pt_core::types::{Adjustment, Bar};
use rusqlite::{params, OptionalExtension};
use rust_decimal::Decimal;

use crate::codec::{dec_to_text, text_to_dec};
use crate::db::Store;
use crate::error::StoreError;

/// One bar together with the series it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarRow {
    pub bar: Bar,
    pub adjustment: Adjustment,
    pub feed: String,
}

/// What [`Store::upsert_bars`] did with each row of a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpsertStats {
    pub inserted: usize,
    pub unchanged: usize,
    pub restated: usize,
}

/// The text stored in `bars.adjustment`.
fn adjustment_text(a: Adjustment) -> &'static str {
    match a {
        Adjustment::Raw => "raw",
        Adjustment::All => "all",
    }
}

/// An instant as fixed-width RFC 3339 UTC, so text order is time order.
fn instant_text(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

/// The values of the latest stored version of a bar.
struct Stored {
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: i64,
}

impl Store {
    /// Stores a batch of bars fetched at `fetched_at`, all or nothing.
    ///
    /// For each row: if its series has no version for that date it is
    /// inserted; if the latest version has the same values it is left
    /// unchanged and no row is written; otherwise a new version is added and
    /// the older ones are kept.
    pub fn upsert_bars(
        &self,
        rows: &[BarRow],
        fetched_at: DateTime<Utc>,
    ) -> Result<UpsertStats, StoreError> {
        let fetched = instant_text(fetched_at);
        let tx = self.conn.unchecked_transaction()?;
        let mut stats = UpsertStats::default();
        {
            let mut latest = tx.prepare(
                "SELECT open, high, low, close, volume FROM bars
                 WHERE symbol = ?1 AND date = ?2 AND adjustment = ?3 AND feed = ?4
                 ORDER BY fetched_at DESC LIMIT 1",
            )?;
            let mut insert = tx.prepare(
                "INSERT INTO bars
                 (symbol, date, adjustment, feed, fetched_at, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for row in rows {
                let b = &row.bar;
                let symbol = b.symbol.as_str();
                let date = b.date.format("%Y-%m-%d").to_string();
                let adjustment = adjustment_text(row.adjustment);
                let volume = i64::try_from(b.volume)
                    .map_err(|_| StoreError::Codec(format!("volume {} too large", b.volume)))?;

                let found = latest
                    .query_row(params![symbol, date, adjustment, row.feed], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                            r.get::<_, i64>(4)?,
                        ))
                    })
                    .optional()?;
                let stored = found
                    .map(|(o, h, l, c, v)| -> Result<Stored, StoreError> {
                        Ok(Stored {
                            open: text_to_dec(&o)?,
                            high: text_to_dec(&h)?,
                            low: text_to_dec(&l)?,
                            close: text_to_dec(&c)?,
                            volume: v,
                        })
                    })
                    .transpose()?;

                match stored {
                    Some(s)
                        if s.open == b.open.value()
                            && s.high == b.high.value()
                            && s.low == b.low.value()
                            && s.close == b.close.value()
                            && s.volume == volume =>
                    {
                        stats.unchanged += 1;
                        continue;
                    }
                    Some(_) => stats.restated += 1,
                    None => stats.inserted += 1,
                }
                insert.execute(params![
                    symbol,
                    date,
                    adjustment,
                    row.feed,
                    fetched,
                    dec_to_text(b.open.value()),
                    dec_to_text(b.high.value()),
                    dec_to_text(b.low.value()),
                    dec_to_text(b.close.value()),
                    volume,
                ])?;
            }
        }
        tx.commit()?;
        Ok(stats)
    }
}
