//! Opening the database and running schema migrations.
//!
//! `PRAGMA user_version` holds the schema version: the number of migrations
//! applied. Each migration runs in its own transaction together with the
//! version bump, so a failed migration leaves the previous version intact.

use std::path::Path;

use rusqlite::Connection;

use crate::error::StoreError;

/// Schema migrations in order. Migration `i` (zero-based) brings the schema
/// to version `i + 1`.
const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_market.sql"),
    include_str!("../migrations/0002_trading.sql"),
    include_str!("../migrations/0003_ops.sql"),
];

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sql(e.to_string())
    }
}

/// A handle on the SQLite store.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens the database at `path`, creating it if absent, and brings the
    /// schema up to date.
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        Store::init(Connection::open(path)?)
    }

    /// Opens a fresh in-memory database with the full schema.
    pub fn open_in_memory() -> Result<Store, StoreError> {
        Store::init(Connection::open_in_memory()?)
    }

    /// The schema version recorded in `PRAGMA user_version`.
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    /// Runs one SQL statement and returns the number of rows changed.
    /// Meant for tests and trigger setup.
    pub fn execute_raw(&self, sql: &str) -> Result<usize, StoreError> {
        Ok(self.conn.execute(sql, [])?)
    }

    fn init(conn: Connection) -> Result<Store, StoreError> {
        // journal_mode returns a row; in-memory databases report "memory".
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<(), StoreError> {
        let current = self.schema_version()?;
        let supported =
            i64::try_from(MIGRATIONS.len()).map_err(|e| StoreError::Sql(e.to_string()))?;
        if current > supported {
            return Err(StoreError::SchemaTooNew {
                found: current,
                supported,
            });
        }
        for (i, sql) in MIGRATIONS.iter().enumerate() {
            let version = i64::try_from(i + 1).map_err(|e| StoreError::Sql(e.to_string()))?;
            if version <= current {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", version)?;
            tx.commit()?;
        }
        Ok(())
    }
}
