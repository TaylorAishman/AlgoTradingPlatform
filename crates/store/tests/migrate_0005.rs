//! R-415: a version-4 database opens, migrates to 5 and keeps its rows.
//! R-416: asset snapshots are append-only at the database level.

use std::path::Path;
use std::str::FromStr;

use chrono::{TimeZone, Utc};
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::market::AssetSnapshot;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

/// Builds a version-4 file the way `db.rs` does: the four migrations, then
/// `PRAGMA user_version = 4`, with one order and one backtest run.
fn write_v4(path: &Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    for sql in [
        include_str!("../migrations/0001_market.sql"),
        include_str!("../migrations/0002_trading.sql"),
        include_str!("../migrations/0003_ops.sql"),
        include_str!("../migrations/0004_append_only.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn.pragma_update(None, "user_version", 4).unwrap();
    conn.execute(
        "INSERT INTO orders (client_order_id, decision_id, symbol, side, qty, leg, state, \
         broker_order_id, filled_qty, reference_price, trade_date) \
         VALUES ('c1', NULL, 'SPY', 'buy', 10, 0, 'intent', NULL, 0, '123.45', '2024-03-01')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO backtest_runs (family, trial_hash, config_hash, created_at) \
         VALUES ('fam', 'th', 'ch', '2024-03-01T00:00:00Z')",
        [],
    )
    .unwrap();
}

fn user_version(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

// R-415
#[test]
fn version_4_database_opens_at_version_5_and_keeps_its_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    write_v4(&path);

    let store = Store::open(&path).expect("open version-4 database");
    assert_eq!(store.schema_version().unwrap(), 5);
    let expected = OrderRow {
        client_order_id: "c1".to_string(),
        decision_id: None,
        symbol: Symbol::new("SPY").unwrap(),
        side: Side::Buy,
        qty: 10,
        leg: 0,
        state: OrderState::Intent,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from_str("123.45").unwrap(),
        trade_date: chrono::NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
    };
    assert_eq!(store.order("c1").unwrap(), Some(expected));
    drop(store);

    let conn = rusqlite::Connection::open(&path).unwrap();
    let off: i64 = conn
        .query_row(
            "SELECT off_ledger FROM orders WHERE client_order_id = 'c1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(off, 0);
    let (cap, src): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT initial_capital, capital_source FROM backtest_runs",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((cap, src), (None, None));
}

// R-415
#[test]
fn reopening_a_migrated_database_stays_at_version_5() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    write_v4(&path);
    drop(Store::open(&path).unwrap());
    let store = Store::open(&path).expect("second open");
    assert_eq!(store.schema_version().unwrap(), 5);
    drop(store);
    assert_eq!(user_version(&path), 5);
}

// R-415, R-416: a new database also lands on version 5.
#[test]
fn new_database_is_at_version_5() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.schema_version().unwrap(), 5);
}

// R-415: the new error variant exists and carries its message.
#[test]
fn invalid_error_carries_its_message() {
    let e = StoreError::Invalid("bad filled_qty".to_string());
    assert!(e.to_string().contains("bad filled_qty"), "got {e}");
}

// R-416
#[test]
fn asset_snapshot_cannot_be_updated_or_deleted_through_another_connection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.db");
    let store = Store::open(&path).unwrap();
    let spy = Symbol::new("SPY").unwrap();
    let t = Utc.with_ymd_and_hms(2024, 3, 1, 14, 30, 0).unwrap();
    let snap = AssetSnapshot {
        symbol: spy.clone(),
        tradable: true,
        shortable: true,
        easy_to_borrow: false,
    };
    store.put_asset_snapshot(&snap, t).unwrap();

    let other = rusqlite::Connection::open(&path).unwrap();
    assert!(other
        .execute("UPDATE asset_snapshots SET tradable = 0", [])
        .is_err());
    assert!(other.execute("DELETE FROM asset_snapshots", []).is_err());

    assert_eq!(store.asset_asof(&spy, None).unwrap(), Some(snap));
}
