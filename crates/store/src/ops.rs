//! Operations records: the log of runs of the binary.

use crate::codec::{dec_to_text, text_to_dec};
use crate::db::Store;
use crate::error::StoreError;
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rust_decimal::Decimal;

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
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM halts WHERE id = ?1)",
                rusqlite::params![id],
                |r| r.get(0),
            )?;
            return Err(StoreError::NotFound(if exists {
                format!("halt {id} is already cleared")
            } else {
                format!("active halt {id}")
            }));
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

/// Convert a stored integer to `u32`, or a `StoreError` when out of range.
fn to_u32(column: &str, v: i64) -> Result<u32, StoreError> {
    u32::try_from(v).map_err(|_| StoreError::Codec(format!("{column} out of range: {v}")))
}

/// One row of the `trials` table: a distinct trial within a strategy family.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialRow {
    pub family: String,
    pub trial_hash: String,
    pub sharpe: f64,
    pub n_obs: u32,
    pub runs: u32,
}

impl Store {
    /// Register a trial. A repeated (family, trial_hash) adds one to `runs` and
    /// updates `sharpe`, but keeps the original `n_obs`, so it counts once.
    pub fn register_trial(&self, trial: &TrialRow) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO trials (family, trial_hash, sharpe, n_obs, runs) \
             VALUES (?1, ?2, ?3, ?4, 1) \
             ON CONFLICT (family, trial_hash) DO UPDATE SET \
             runs = runs + 1, sharpe = excluded.sharpe",
            rusqlite::params![trial.family, trial.trial_hash, trial.sharpe, trial.n_obs],
        )?;
        Ok(())
    }

    /// Every trial of a family, ordered by hash.
    pub fn trials(&self, family: &str) -> Result<Vec<TrialRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT family, trial_hash, sharpe, n_obs, runs FROM trials \
             WHERE family = ?1 ORDER BY trial_hash",
        )?;
        let rows = stmt.query_map([family], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (family, trial_hash, sharpe, n_obs, runs) = row?;
            out.push(TrialRow {
                family,
                trial_hash,
                sharpe,
                n_obs: to_u32("n_obs", n_obs)?,
                runs: to_u32("runs", runs)?,
            });
        }
        Ok(out)
    }
}

impl Store {
    /// Save a backtest and its equity curve in one transaction; returns the id.
    pub fn save_backtest(
        &self,
        family: &str,
        trial_hash: &str,
        config_hash: &str,
        equity: &[(NaiveDate, Decimal)],
        at: DateTime<Utc>,
    ) -> Result<i64, StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO backtest_runs (family, trial_hash, config_hash, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![family, trial_hash, config_hash, instant_to_text(at)],
        )?;
        let id = tx.last_insert_rowid();
        {
            let mut stmt = tx.prepare(
                "INSERT INTO backtest_equity (run_id, date, equity) VALUES (?1, ?2, ?3)",
            )?;
            for (date, value) in equity {
                stmt.execute(rusqlite::params![
                    id,
                    date.format("%Y-%m-%d").to_string(),
                    dec_to_text(*value)
                ])?;
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// The equity curve of a saved backtest, in date order.
    pub fn backtest_equity(&self, id: i64) -> Result<Vec<(NaiveDate, Decimal)>, StoreError> {
        let known: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM backtest_runs WHERE id = ?1",
            [id],
            |r| r.get(0),
        )?;
        if known == 0 {
            return Err(StoreError::NotFound(format!("backtest {id}")));
        }
        let mut stmt = self
            .conn
            .prepare("SELECT date, equity FROM backtest_equity WHERE run_id = ?1 ORDER BY date")?;
        let rows = stmt.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (date, equity) = row?;
            let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .map_err(|e| StoreError::Codec(format!("invalid date {date:?}: {e}")))?;
            out.push((date, text_to_dec(&equity)?));
        }
        Ok(out)
    }

    /// The strategy family of a saved backtest, read from its run row.
    pub fn backtest_family(&self, id: i64) -> Result<String, StoreError> {
        match self.conn.query_row(
            "SELECT family FROM backtest_runs WHERE id = ?1",
            [id],
            |r| r.get::<_, String>(0),
        ) {
            Ok(family) => Ok(family),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(StoreError::NotFound(format!("backtest {id}")))
            }
            Err(e) => Err(e.into()),
        }
    }
}
