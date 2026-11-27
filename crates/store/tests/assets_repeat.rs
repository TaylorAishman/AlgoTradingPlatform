//! R-419: a second asset snapshot for the same symbol and instant must not
//! replace the first.

use chrono::{DateTime, TimeZone, Utc};
use pt_core::types::Symbol;
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_store::market::AssetSnapshot;

fn t1() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 10, 0, 0).unwrap()
}

fn spy(tradable: bool) -> AssetSnapshot {
    AssetSnapshot {
        symbol: Symbol::new("SPY").unwrap(),
        tradable,
        shortable: true,
        easy_to_borrow: true,
    }
}

#[test]
fn same_snapshot_twice_is_ok_both_times() {
    // R-419
    let s = Store::open_in_memory().unwrap();
    s.put_asset_snapshot(&spy(true), t1()).unwrap();
    s.put_asset_snapshot(&spy(true), t1()).unwrap();
}

#[test]
fn a_different_snapshot_at_the_same_instant_is_a_duplicate_and_keeps_the_first() {
    // R-419
    let s = Store::open_in_memory().unwrap();
    s.put_asset_snapshot(&spy(true), t1()).unwrap();
    s.put_asset_snapshot(&spy(true), t1()).unwrap();
    let err = s.put_asset_snapshot(&spy(false), t1()).unwrap_err();
    assert!(matches!(err, StoreError::Duplicate(_)), "got {err:?}");
    let a = s
        .asset_asof(&Symbol::new("SPY").unwrap(), None)
        .unwrap()
        .unwrap();
    assert!(a.tradable);
}
