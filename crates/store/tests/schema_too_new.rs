//! R-051: a database newer than the binary is refused.

use pt_store::db::Store;
use pt_store::error::StoreError;

#[test]
fn opening_a_database_with_a_newer_version_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let store = Store::open(&path).expect("open new database");
    store.execute_raw("PRAGMA user_version = 9999").unwrap();
    drop(store);

    match Store::open(&path) {
        Err(StoreError::SchemaTooNew { found, supported }) => {
            assert_eq!(found, 9999);
            assert!((1..9999).contains(&supported));
        }
        other => panic!("expected SchemaTooNew, got {other:?}"),
    }
}

#[test]
fn refusal_leaves_the_newer_version_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let store = Store::open(&path).unwrap();
    store.execute_raw("PRAGMA user_version = 9999").unwrap();
    drop(store);

    assert!(Store::open(&path).is_err());
    let conn = rusqlite::Connection::open(&path).unwrap();
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 9999);
}
