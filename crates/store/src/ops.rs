//! Operations records: the log of runs of the binary.

use crate::db::Store;
use crate::error::StoreError;
use chrono::{DateTime, SecondsFormat, Utc};

/// One row of the `runs` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub stage: String,
    pub status: String,
    pub config_hash: String,
    pub code_version: String,
    pub phases: String,
}

/// One row of the `halts` table. A halt is active while `cleared_at` is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaltRow {
    pub id: i64,
    pub reason: String,
    pub detail: String,
    pub set_at: DateTime<Utc>,
    pub cleared_at: Option<DateTime<Utc>>,
    pub clear_note: Option<String>,
}

fn instant_to_text(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn text_to_instant(s: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| StoreError::Codec(format!("invalid instant {s:?}: {e}")))
}

impl Store {
    /// Record the start of a run and return its id. The run is `running` until finished.
    pub fn start_run(
        &self,
        stage: &str,
        config_hash: &str,
        code_version: &str,
        started_at: DateTime<Utc>,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO runs (started_at, stage, status, config_hash, code_version, phases) \
             VALUES (?1, ?2, 'running', ?3, ?4, '')",
            rusqlite::params![
                instant_to_text(started_at),
                stage,
                config_hash,
                code_version
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record the end of a run with its status and phases.
    pub fn finish_run(
        &self,
        id: i64,
        status: &str,
        phases: &str,
        finished_at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let n = self.conn.execute(
            "UPDATE runs SET status = ?1, phases = ?2, finished_at = ?3 WHERE id = ?4",
            rusqlite::params![status, phases, instant_to_text(finished_at), id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("run {id}")));
        }
        Ok(())
    }

    /// All runs, oldest first.
    pub fn runs(&self) -> Result<Vec<RunRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, started_at, finished_at, stage, status, config_hash, code_version, phases \
             FROM runs ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, started, finished, stage, status, config_hash, code_version, phases) = row?;
            out.push(RunRow {
                id,
                started_at: text_to_instant(&started)?,
                finished_at: finished.as_deref().map(text_to_instant).transpose()?,
                stage,
                status,
                config_hash,
                code_version,
                phases,
            });
        }
        Ok(out)
    }

    /// Set a halt and return its id. It stays active until cleared.
    pub fn set_halt(
        &self,
        reason: &str,
        detail: &str,
        at: DateTime<Utc>,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO halts (reason, detail, set_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![reason, detail, instant_to_text(at)],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Clear an active halt with a note. The row stays as history.
    pub fn clear_halt(&self, id: i64, note: &str, at: DateTime<Utc>) -> Result<(), StoreError> {
        let n = self.conn.execute(
            "UPDATE halts SET cleared_at = ?1, clear_note = ?2 \
             WHERE id = ?3 AND cleared_at IS NULL",
            rusqlite::params![instant_to_text(at), note, id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound(format!("active halt {id}")));
        }
        Ok(())
    }

    /// Halts not yet cleared, oldest first.
    pub fn active_halts(&self) -> Result<Vec<HaltRow>, StoreError> {
        self.query_halts("WHERE cleared_at IS NULL")
    }

    /// Every halt, cleared or not, oldest first.
    pub fn halts(&self) -> Result<Vec<HaltRow>, StoreError> {
        self.query_halts("")
    }

    fn query_halts(&self, filter: &str) -> Result<Vec<HaltRow>, StoreError> {
        let sql = format!(
            "SELECT id, reason, detail, set_at, cleared_at, clear_note FROM halts {filter} ORDER BY id"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, reason, detail, set, cleared, clear_note) = row?;
            out.push(HaltRow {
                id,
                reason,
                detail,
                set_at: text_to_instant(&set)?,
                cleared_at: cleared.as_deref().map(text_to_instant).transpose()?,
                clear_note,
            });
        }
        Ok(out)
    }

    /// Record an alert and return its id.
    pub fn record_alert(
        &self,
        level: &str,
        code: &str,
        message: &str,
        delivered: bool,
        at: DateTime<Utc>,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO alerts (level, code, message, delivered, at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![level, code, message, delivered, instant_to_text(at)],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// All alerts as (level, code, message, delivered), oldest first.
    pub fn alerts(&self) -> Result<Vec<(String, String, String, bool)>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT level, code, message, delivered FROM alerts ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}
