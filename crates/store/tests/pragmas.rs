//! R-052: the store uses write-ahead logging and enforces foreign keys.

use pt_store::db::Store;
use pt_store::error::StoreError;
use rusqlite::Connection;

// R-052
#[test]
fn file_store_uses_write_ahead_logging() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let _store = Store::open(&path).expect("open new database");
    let other = Connection::open(&path).unwrap();
    let mode: String = other
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

// R-052
#[test]
fn order_referencing_a_missing_decision_is_rejected() {
    let store = Store::open_in_memory().unwrap();
    let err = store
        .execute_raw(
            "INSERT INTO orders (client_order_id, decision_id, symbol, side, qty, leg, state, \
             filled_qty, reference_price, trade_date) \
             VALUES ('c1', 12345, 'SPY', 'buy', 1, 0, 'new', 0, '100', '2024-01-02')",
        )
        .expect_err("foreign key must be enforced");
    assert!(matches!(err, StoreError::Sql(_)), "got {err:?}");
}

// R-052
#[test]
fn order_referencing_an_existing_decision_is_accepted() {
    let store = Store::open_in_memory().unwrap();
    store
        .execute_raw(
            "INSERT INTO decisions (id, strategy_id, signal_date, mode, status, inputs_json, \
             outputs_json, created_at) \
             VALUES (12345, 's', '2024-01-01', 'paper', 'ok', '{}', '{}', '2024-01-01T00:00:00Z')",
        )
        .unwrap();
    let n = store
        .execute_raw(
            "INSERT INTO orders (client_order_id, decision_id, symbol, side, qty, leg, state, \
             filled_qty, reference_price, trade_date) \
             VALUES ('c1', 12345, 'SPY', 'buy', 1, 0, 'new', 0, '100', '2024-01-02')",
        )
        .unwrap();
    assert_eq!(n, 1);
}
