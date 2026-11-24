//! R-050: opening a store creates the schema and is repeatable.

use pt_store::db::Store;

// R-050
#[test]
fn open_on_new_path_creates_schema_at_version_one_or_more() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let store = Store::open(&path).expect("open new database");
    assert!(store.schema_version().unwrap() >= 1);
}

// R-050
#[test]
fn reopening_the_same_path_succeeds_with_the_same_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let first = {
        let store = Store::open(&path).expect("first open");
        store.schema_version().unwrap()
    };
    let store = Store::open(&path).expect("second open");
    assert_eq!(store.schema_version().unwrap(), first);
}

// R-050
#[test]
fn in_memory_store_has_the_same_version_as_a_file_store() {
    let dir = tempfile::tempdir().unwrap();
    let file = Store::open(&dir.path().join("store.db")).unwrap();
    let mem = Store::open_in_memory().expect("open in memory");
    assert!(mem.schema_version().unwrap() >= 1);
    assert_eq!(
        mem.schema_version().unwrap(),
        file.schema_version().unwrap()
    );
}

// R-050
#[test]
fn migration_creates_the_market_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    drop(Store::open(&path).unwrap());
    let conn = rusqlite::Connection::open(&path).unwrap();
    for table in ["bars", "calendar_sessions", "asset_snapshots"] {
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
fn execute_raw_runs_sql_against_the_store() {
    let store = Store::open_in_memory().unwrap();
    store
        .execute_raw("CREATE TABLE scratch (x INTEGER)")
        .unwrap();
    let n = store
        .execute_raw("INSERT INTO scratch (x) VALUES (1)")
        .unwrap();
    assert_eq!(n, 1);
}
