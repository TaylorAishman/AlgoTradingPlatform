//! Shadow comparison rows and slippage rows.

use crate::codec::{dec_to_text, text_to_dec};
use crate::db::Store;
use crate::error::StoreError;
use chrono::NaiveDate;
use rusqlite::params;
use rust_decimal::Decimal;

const DATE_FORMAT: &str = "%Y-%m-%d";

/// One day of paper return against shadow return.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowDay {
    pub date: NaiveDate,
    pub paper_return: f64,
    pub shadow_return: f64,
    pub diff: f64,
}

/// Fill price against reference price for one order.
#[derive(Debug, Clone, PartialEq)]
pub struct SlippageRow {
    pub client_order_id: String,
    pub reference_price: Decimal,
    pub avg_fill_price: Decimal,
    pub qty: u32,
    pub bps: f64,
}

impl Store {
    /// Insert a shadow day, replacing any row for the same date.
    pub fn put_shadow_day(&self, day: &ShadowDay) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO shadow_daily (date, paper_return, shadow_return, diff) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                day.date.format(DATE_FORMAT).to_string(),
                day.paper_return,
                day.shadow_return,
                day.diff
            ],
        )?;
        Ok(())
    }

    /// All shadow days, oldest first.
    pub fn shadow_days(&self) -> Result<Vec<ShadowDay>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT date, paper_return, shadow_return, diff FROM shadow_daily ORDER BY date",
        )?;
        let raw = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, f64>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        raw.into_iter()
            .map(|(d, paper_return, shadow_return, diff)| {
                let date = NaiveDate::parse_from_str(&d, DATE_FORMAT)
                    .map_err(|e| StoreError::Codec(format!("invalid date {d:?}: {e}")))?;
                Ok(ShadowDay {
                    date,
                    paper_return,
                    shadow_return,
                    diff,
                })
            })
            .collect()
    }

    /// Insert a slippage row, replacing any row for the same client order id.
    pub fn put_slippage(&self, row: &SlippageRow) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO slippage \
             (client_order_id, reference_price, avg_fill_price, qty, bps) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                row.client_order_id,
                dec_to_text(row.reference_price),
                dec_to_text(row.avg_fill_price),
                row.qty,
                row.bps
            ],
        )?;
        Ok(())
    }

    /// All slippage rows, in client order id order.
    pub fn slippage_rows(&self) -> Result<Vec<SlippageRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT client_order_id, reference_price, avg_fill_price, qty, bps \
             FROM slippage ORDER BY client_order_id",
        )?;
        let raw = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, u32>(3)?,
                    r.get::<_, f64>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        raw.into_iter()
            .map(|(client_order_id, rp, ap, qty, bps)| {
                Ok(SlippageRow {
                    client_order_id,
                    reference_price: text_to_dec(&rp)?,
                    avg_fill_price: text_to_dec(&ap)?,
                    qty,
                    bps,
                })
            })
            .collect()
    }
}
