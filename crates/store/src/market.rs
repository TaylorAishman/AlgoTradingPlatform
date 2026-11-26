//! Market data repositories: versioned daily bars.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use pt_calendar::calendar::TradingCalendar;
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
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

/// The broker's flags for one asset, as fetched at some instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSnapshot {
    pub symbol: Symbol,
    pub tradable: bool,
    pub shortable: bool,
    pub easy_to_borrow: bool,
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
        let mut seen: std::collections::HashMap<(&str, chrono::NaiveDate, &str, &str), &BarRow> =
            std::collections::HashMap::new();
        for row in rows {
            let b = &row.bar;
            let key = (
                b.symbol.as_str(),
                b.date,
                adjustment_text(row.adjustment),
                row.feed.as_str(),
            );
            if let Some(first) = seen.insert(key, row) {
                let f = &first.bar;
                let same = f.open.value() == b.open.value()
                    && f.high.value() == b.high.value()
                    && f.low.value() == b.low.value()
                    && f.close.value() == b.close.value()
                    && f.volume == b.volume;
                if !same {
                    return Err(StoreError::Duplicate(format!(
                        "two different bars for {} on {} ({}, {}) in one batch",
                        b.symbol.as_str(),
                        b.date.format("%Y-%m-%d"),
                        adjustment_text(row.adjustment),
                        row.feed
                    )));
                }
            }
        }
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

    /// Reads bars for `from..=to`, one per date, ordered by date.
    ///
    /// Each date yields its latest version fetched at or before `asof`, or
    /// its latest version overall when `asof` is `None`.
    pub fn bars_asof(
        &self,
        symbol: &Symbol,
        adjustment: Adjustment,
        feed: &str,
        from: NaiveDate,
        to: NaiveDate,
        asof: Option<DateTime<Utc>>,
    ) -> Result<Vec<Bar>, StoreError> {
        let asof = asof.map(instant_text);
        let mut stmt = self.conn.prepare(
            "SELECT b.date, b.open, b.high, b.low, b.close, b.volume FROM bars b
             WHERE b.symbol = ?1 AND b.adjustment = ?2 AND b.feed = ?3
               AND b.date >= ?4 AND b.date <= ?5
               AND (?6 IS NULL OR b.fetched_at <= ?6)
               AND b.fetched_at = (
                   SELECT MAX(x.fetched_at) FROM bars x
                   WHERE x.symbol = b.symbol AND x.date = b.date
                     AND x.adjustment = b.adjustment AND x.feed = b.feed
                     AND (?6 IS NULL OR x.fetched_at <= ?6))
             ORDER BY b.date",
        )?;
        let rows = stmt.query_map(
            params![
                symbol.as_str(),
                adjustment_text(adjustment),
                feed,
                from.format("%Y-%m-%d").to_string(),
                to.format("%Y-%m-%d").to_string(),
                asof,
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                ))
            },
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (date, o, h, l, c, v) = row?;
            let price = |t: &str| -> Result<Price, StoreError> {
                Price::new(text_to_dec(t)?).map_err(|e| StoreError::Codec(e.to_string()))
            };
            out.push(Bar {
                symbol: symbol.clone(),
                date: NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|e| StoreError::Codec(format!("bad date {date}: {e}")))?,
                open: price(&o)?,
                high: price(&h)?,
                low: price(&l)?,
                close: price(&c)?,
                volume: u64::try_from(v)
                    .map_err(|_| StoreError::Codec(format!("negative volume {v}")))?,
            });
        }
        Ok(out)
    }

    /// Stores trading session dates, all or nothing; dates already stored
    /// are left as they are.
    pub fn put_sessions(&self, dates: &[NaiveDate]) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut insert =
                tx.prepare("INSERT OR IGNORE INTO calendar_sessions (date) VALUES (?1)")?;
            for d in dates {
                insert.execute(params![d.format("%Y-%m-%d").to_string()])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// The calendar of every stored session; `NotFound` when there are none.
    pub fn calendar(&self) -> Result<TradingCalendar, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT date FROM calendar_sessions ORDER BY date")?;
        let texts = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let mut dates = Vec::new();
        for t in texts {
            let t = t?;
            dates.push(
                NaiveDate::parse_from_str(&t, "%Y-%m-%d")
                    .map_err(|e| StoreError::Codec(format!("bad date {t}: {e}")))?,
            );
        }
        if dates.is_empty() {
            return Err(StoreError::NotFound("calendar sessions".to_string()));
        }
        TradingCalendar::new(dates).map_err(|e| StoreError::Codec(e.to_string()))
    }

    /// Stores an asset snapshot fetched at `fetched_at`. A snapshot for the
    /// same symbol and instant is replaced.
    pub fn put_asset_snapshot(
        &self,
        snapshot: &AssetSnapshot,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO asset_snapshots
             (symbol, fetched_at, tradable, shortable, easy_to_borrow)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                snapshot.symbol.as_str(),
                instant_text(fetched_at),
                snapshot.tradable,
                snapshot.shortable,
                snapshot.easy_to_borrow,
            ],
        )?;
        Ok(())
    }

    /// The latest snapshot fetched at or before `asof`, or the latest overall
    /// when `asof` is `None`.
    pub fn asset_asof(
        &self,
        symbol: &Symbol,
        asof: Option<DateTime<Utc>>,
    ) -> Result<Option<AssetSnapshot>, StoreError> {
        let asof = asof.map(instant_text);
        let found = self
            .conn
            .query_row(
                "SELECT tradable, shortable, easy_to_borrow FROM asset_snapshots
                 WHERE symbol = ?1 AND (?2 IS NULL OR fetched_at <= ?2)
                 ORDER BY fetched_at DESC LIMIT 1",
                params![symbol.as_str(), asof],
                |r| {
                    Ok((
                        r.get::<_, bool>(0)?,
                        r.get::<_, bool>(1)?,
                        r.get::<_, bool>(2)?,
                    ))
                },
            )
            .optional()?;
        Ok(
            found.map(|(tradable, shortable, easy_to_borrow)| AssetSnapshot {
                symbol: symbol.clone(),
                tradable,
                shortable,
                easy_to_borrow,
            }),
        )
    }
}
