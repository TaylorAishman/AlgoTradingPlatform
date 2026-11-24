//! R-050: the trading tables are created by the migration runner.

use std::collections::BTreeSet;
use std::path::Path;

use pt_store::db::Store;
use rusqlite::Connection;

fn fresh_db(dir: &Path) -> Connection {
    let path = dir.join("store.db");
    drop(Store::open(&path).expect("open new database"));
    Connection::open(&path).unwrap()
}

fn columns(conn: &Connection, table: &str) -> BTreeSet<String> {
    let mut stmt = conn
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn primary_key(conn: &Connection, table: &str) -> BTreeSet<String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name FROM pragma_table_info('{table}') WHERE pk > 0"
        ))
        .unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Column sets of every unique index (including the implicit ones from
/// UNIQUE constraints) on `table`.
fn unique_indexes(conn: &Connection, table: &str) -> Vec<BTreeSet<String>> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT name FROM pragma_index_list('{table}') WHERE \"unique\" = 1"
        ))
        .unwrap();
    let names: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    names
        .iter()
        .map(|n| {
            let mut s = conn
                .prepare(&format!("SELECT name FROM pragma_index_info('{n}')"))
                .unwrap();
            s.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        })
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

// R-050
#[test]
fn migration_creates_the_trading_tables() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    for table in [
        "decisions",
        "orders",
        "order_events",
        "fills",
        "ledger_entries",
        "equity_snapshots",
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
fn schema_version_covers_the_trading_migration() {
    let store = Store::open_in_memory().unwrap();
    assert!(store.schema_version().unwrap() >= 2);
}

// R-050
#[test]
fn in_memory_store_has_the_trading_tables() {
    let store = Store::open_in_memory().unwrap();
    for table in ["decisions", "orders", "order_events", "fills"] {
        let n = store
            .execute_raw(&format!("DELETE FROM {table}"))
            .unwrap_or_else(|e| panic!("table {table} should exist: {e}"));
        assert_eq!(n, 0);
    }
}

// R-050
#[test]
fn reopening_keeps_the_trading_tables_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let first = Store::open(&path).unwrap().schema_version().unwrap();
    let again = Store::open(&path).expect("second open must not fail");
    assert_eq!(again.schema_version().unwrap(), first);
    drop(again);
    let conn = Connection::open(&path).unwrap();
    assert!(!columns(&conn, "orders").is_empty());
}

// R-050
#[test]
fn decisions_has_the_decision_columns() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let cols = columns(&conn, "decisions");
    for c in [
        "id",
        "strategy_id",
        "signal_date",
        "trade_date",
        "mode",
        "status",
        "inputs_json",
        "outputs_json",
        "created_at",
    ] {
        assert!(cols.contains(c), "decisions lacks column {c}: {cols:?}");
    }
}

// R-050
#[test]
fn decisions_are_unique_per_strategy_signal_date_and_mode() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let want = set(&["strategy_id", "signal_date", "mode"]);
    assert!(
        unique_indexes(&conn, "decisions").contains(&want),
        "decisions needs a unique key on (strategy_id, signal_date, mode)"
    );
}

// R-050
#[test]
fn orders_has_the_order_columns_keyed_by_client_order_id() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let cols = columns(&conn, "orders");
    for c in [
        "client_order_id",
        "decision_id",
        "symbol",
        "side",
        "qty",
        "leg",
        "state",
        "broker_order_id",
        "filled_qty",
        "reference_price",
        "trade_date",
    ] {
        assert!(cols.contains(c), "orders lacks column {c}: {cols:?}");
    }
    assert_eq!(primary_key(&conn, "orders"), set(&["client_order_id"]));
}

// R-050
#[test]
fn orders_decision_id_references_decisions() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let target: String = conn
        .query_row(
            "SELECT \"table\" FROM pragma_foreign_key_list('orders') WHERE \"from\" = 'decision_id'",
            [],
            |r| r.get(0),
        )
        .expect("orders.decision_id should be a foreign key");
    assert_eq!(target, "decisions");
}

// R-050
#[test]
fn order_events_has_a_single_integer_key() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    assert_eq!(primary_key(&conn, "order_events").len(), 1);
    assert!(!columns(&conn, "order_events").is_empty());
}

// R-050
#[test]
fn fills_has_the_fill_columns_keyed_by_activity_id() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let cols = columns(&conn, "fills");
    for c in [
        "activity_id",
        "broker_order_id",
        "symbol",
        "side",
        "qty",
        "price",
        "at",
    ] {
        assert!(cols.contains(c), "fills lacks column {c}: {cols:?}");
    }
    assert_eq!(primary_key(&conn, "fills"), set(&["activity_id"]));
}

// R-050
#[test]
fn ledger_entries_has_the_ledger_columns_and_an_integer_key() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let cols = columns(&conn, "ledger_entries");
    for c in [
        "book",
        "date",
        "kind",
        "symbol",
        "qty",
        "price",
        "cash_delta",
        "ref_id",
    ] {
        assert!(
            cols.contains(c),
            "ledger_entries lacks column {c}: {cols:?}"
        );
    }
    assert_eq!(primary_key(&conn, "ledger_entries").len(), 1);
}

// R-050
#[test]
fn equity_snapshots_is_keyed_by_book_and_date() {
    let dir = tempfile::tempdir().unwrap();
    let conn = fresh_db(dir.path());
    let cols = columns(&conn, "equity_snapshots");
    for c in [
        "book",
        "date",
        "cash",
        "positions_value",
        "equity",
        "recorded_at",
    ] {
        assert!(
            cols.contains(c),
            "equity_snapshots lacks column {c}: {cols:?}"
        );
    }
    assert_eq!(
        primary_key(&conn, "equity_snapshots"),
        set(&["book", "date"])
    );
}
