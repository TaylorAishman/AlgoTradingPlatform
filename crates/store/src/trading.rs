//! Repositories for decisions, orders, fills and the ledgers.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::{params, Row};

use crate::db::Store;
use crate::error::StoreError;

/// Whether a decision was only computed (`DryRun`) or sent to the broker (`Paper`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    DryRun,
    Paper,
}

impl Mode {
    fn as_text(self) -> &'static str {
        match self {
            Mode::DryRun => "dry_run",
            Mode::Paper => "paper",
        }
    }

    fn from_text(s: &str) -> Result<Mode, StoreError> {
        match s {
            "dry_run" => Ok(Mode::DryRun),
            "paper" => Ok(Mode::Paper),
            other => Err(StoreError::Codec(format!("invalid mode {other:?}"))),
        }
    }
}

/// One decision: the inputs and outputs of a strategy on a signal date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRow {
    pub id: i64,
    pub strategy_id: String,
    pub signal_date: NaiveDate,
    pub trade_date: Option<NaiveDate>,
    pub mode: Mode,
    pub status: String,
    pub inputs_json: String,
    pub outputs_json: String,
    pub created_at: DateTime<Utc>,
}

const DECISION_COLUMNS: &str =
    "id, strategy_id, signal_date, trade_date, mode, status, inputs_json, outputs_json, created_at";

fn date_to_text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn text_to_date(s: &str) -> Result<NaiveDate, StoreError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| StoreError::Codec(format!("invalid date {s:?}: {e}")))
}

fn time_to_text(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn text_to_time(s: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| StoreError::Codec(format!("invalid timestamp {s:?}: {e}")))
}

fn decision_from_row(r: &Row<'_>) -> Result<DecisionRow, StoreError> {
    let trade_date: Option<String> = r.get(3)?;
    Ok(DecisionRow {
        id: r.get(0)?,
        strategy_id: r.get(1)?,
        signal_date: text_to_date(&r.get::<_, String>(2)?)?,
        trade_date: trade_date.as_deref().map(text_to_date).transpose()?,
        mode: Mode::from_text(&r.get::<_, String>(4)?)?,
        status: r.get(5)?,
        inputs_json: r.get(6)?,
        outputs_json: r.get(7)?,
        created_at: text_to_time(&r.get::<_, String>(8)?)?,
    })
}

fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation
                && f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    )
}

impl Store {
    /// Inserts a decision and returns its id. The `id` field of `d` is ignored.
    /// A second decision for the same strategy, signal date and mode is
    /// `StoreError::Duplicate`.
    pub fn insert_decision(&self, d: &DecisionRow) -> Result<i64, StoreError> {
        let res = self.conn.execute(
            "INSERT INTO decisions (strategy_id, signal_date, trade_date, mode, status, \
             inputs_json, outputs_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                d.strategy_id,
                date_to_text(d.signal_date),
                d.trade_date.map(date_to_text),
                d.mode.as_text(),
                d.status,
                d.inputs_json,
                d.outputs_json,
                time_to_text(d.created_at),
            ],
        );
        match res {
            Ok(_) => Ok(self.conn.last_insert_rowid()),
            Err(e) if is_unique_violation(&e) => Err(StoreError::Duplicate(format!(
                "decision for {} on {} in mode {}",
                d.strategy_id,
                date_to_text(d.signal_date),
                d.mode.as_text()
            ))),
            Err(e) => Err(e.into()),
        }
    }

    /// The decision for a strategy, signal date and mode, if one is stored.
    pub fn decision_for(
        &self,
        strategy_id: &str,
        signal_date: NaiveDate,
        mode: Mode,
    ) -> Result<Option<DecisionRow>, StoreError> {
        let sql = format!(
            "SELECT {DECISION_COLUMNS} FROM decisions \
             WHERE strategy_id = ?1 AND signal_date = ?2 AND mode = ?3"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![
            strategy_id,
            date_to_text(signal_date),
            mode.as_text()
        ])?;
        match rows.next()? {
            Some(r) => Ok(Some(decision_from_row(r)?)),
            None => Ok(None),
        }
    }

    /// All decisions of one mode, in insertion order.
    pub fn decisions(&self, mode: Mode) -> Result<Vec<DecisionRow>, StoreError> {
        let sql = format!("SELECT {DECISION_COLUMNS} FROM decisions WHERE mode = ?1 ORDER BY id");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![mode.as_text()])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(decision_from_row(r)?);
        }
        Ok(out)
    }
}
