//! R-050: the shadow and operations tables are created by the migration runner.

use std::collections::BTreeSet;
use std::path::Path;

use pt_store::db::Store;
use rusqlite::Connection;

fn fresh_db(dir: &Path) -> Connection {
    let path = dir.join("store.db");
    drop(Store::open(&path).expect("open new database"));
    Connection::open(&path).unwrap()
}

fn names(conn: &Connection, sql: &str) -> BTreeSet<String> {
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn columns(conn: &Connection, table: &str) -> BTreeSet<String> {
    names(
        conn,
        &format!("SELECT name FROM pragma_table_info('{table}')"),
    )
}

fn primary_key(conn: &Connection, table: &str) -> BTreeSet<String> {
    names(
        conn,
        &format!("SELECT name FROM pragma_table_info('{table}') WHERE pk > 0"),
    )
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn assert_columns(conn: &Connection, table: &str, wanted: &[&str]) {
    let cols = columns(conn, table);
    for c in wanted {
        assert!(cols.contains(*c), "{table} lacks column {c}: {cols:?}");
    }
}

// R-050
#[test]
fn migration_creates_the_shadow_and_operations_tables() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    for table in [
        "shadow_daily",
        "slippage",
        "runs",
        "halts",
        "alerts",
        "trials",
        "backtest_runs",
        "backtest_equity",
    ] {
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "table {table} should exist");
    }
}

// R-050
#[test]
fn schema_version_covers_the_ops_migration() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.schema_version().unwrap() >= 3);
}

// R-050
#[test]
fn in_memory_store_has_the_ops_tables() {
    let store = Store::open_in_memory().unwrap();
    for table in ["shadow_daily", "slippage", "runs", "halts", "alerts"] {
        let n = store
            .execute_raw(&format!("DELETE FROM {table}"))
            .unwrap_or_else(|e| panic!("table {table} should exist: {e}"));
        assert_eq!(n, 0);
    }
}

// R-050
#[test]
fn reopening_keeps_the_ops_tables_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let first = Store::open(&path).unwrap().schema_version().unwrap();
    let again = Store::open(&path).expect("second open must not fail");
    assert_eq!(again.schema_version().unwrap(), first);
    drop(again);
    let conn = Connection::open(&path).unwrap();
    assert!(!columns(&conn, "runs").is_empty());
}

// R-050
#[test]
fn shadow_daily_is_keyed_by_date() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_columns(
        &conn,
        "shadow_daily",
        &["date", "paper_return", "shadow_return", "diff"],
    );
    assert_eq!(primary_key(&conn, "shadow_daily"), set(&["date"]));
}

// R-050
#[test]
fn slippage_is_keyed_by_client_order_id() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_columns(
        &conn,
        "slippage",
        &[
            "client_order_id",
            "reference_price",
            "avg_fill_price",
            "qty",
            "bps",
        ],
    );
    assert_eq!(primary_key(&conn, "slippage"), set(&["client_order_id"]));
}

// R-050
#[test]
fn runs_has_the_run_columns_and_an_integer_key() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_columns(
        &conn,
        "runs",
        &[
            "id",
            "started_at",
            "finished_at",
            "stage",
            "status",
            "config_hash",
            "code_version",
            "phases",
        ],
    );
    assert_eq!(primary_key(&conn, "runs"), set(&["id"]));
}

// R-050
#[test]
fn halts_has_the_halt_columns_and_an_integer_key() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_columns(
        &conn,
        "halts",
        &[
            "id",
            "reason",
            "detail",
            "set_at",
            "cleared_at",
            "clear_note",
        ],
    );
    assert_eq!(primary_key(&conn, "halts"), set(&["id"]));
}

// R-050
#[test]
fn alerts_has_the_alert_columns_and_a_single_key() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_columns(&conn, "alerts", &["level", "code", "message", "delivered"]);
    assert_eq!(primary_key(&conn, "alerts").len(), 1);
}

// R-050
#[test]
fn trials_is_keyed_by_family_and_trial_hash() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_columns(
        &conn,
        "trials",
        &["family", "trial_hash", "sharpe", "n_obs", "runs"],
    );
    assert_eq!(primary_key(&conn, "trials"), set(&["family", "trial_hash"]));
}

// R-050
#[test]
fn backtest_tables_are_keyed_by_run_id_and_run_id_with_date() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_eq!(primary_key(&conn, "backtest_runs").len(), 1);
    assert_columns(&conn, "backtest_equity", &["run_id", "date"]);
    assert_eq!(
        primary_key(&conn, "backtest_equity"),
        set(&["run_id", "date"])
    );
}
