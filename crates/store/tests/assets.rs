//! R-059: asset snapshots are readable as of an instant.

use chrono::{DateTime, TimeZone, Utc};
use pt_core::types::Symbol;
use pt_store::db::Store;
use pt_store::market::AssetSnapshot;

fn t(h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, h, 0, 0).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn snap(s: &str, shortable: bool) -> AssetSnapshot {
    AssetSnapshot {
        symbol: sym(s),
        tradable: true,
        shortable,
        easy_to_borrow: shortable,
    }
}

fn seeded() -> Store {
    let s = Store::open_in_memory().unwrap();
    s.put_asset_snapshot(&snap("SPY", true), t(10)).unwrap();
    s.put_asset_snapshot(&snap("SPY", false), t(12)).unwrap();
    s
}

#[test]
fn latest_snapshot_is_returned_without_asof() {
    // R-059
    let s = seeded();
    let a = s.asset_asof(&sym("SPY"), None).unwrap().unwrap();
    assert!(!a.shortable);
    assert_eq!(a, snap("SPY", false));
}

#[test]
fn snapshot_as_of_the_earlier_instant_is_the_earlier_version() {
    // R-059: asof is inclusive.
    let s = seeded();
    let a = s.asset_asof(&sym("SPY"), Some(t(10))).unwrap().unwrap();
    assert!(a.shortable);
    assert!(a.easy_to_borrow);
    assert_eq!(a, snap("SPY", true));
}

#[test]
fn snapshot_between_versions_is_the_earlier_one() {
    // R-059
    let s = seeded();
    let a = s.asset_asof(&sym("SPY"), Some(t(11))).unwrap().unwrap();
    assert!(a.shortable);
}

#[test]
fn snapshot_as_of_before_the_first_is_none() {
    // R-059
    let s = seeded();
    assert_eq!(s.asset_asof(&sym("SPY"), Some(t(9))).unwrap(), None);
}

#[test]
fn unknown_symbol_is_none() {
    // R-059
    let s = seeded();
    assert_eq!(s.asset_asof(&sym("QQQ"), None).unwrap(), None);
}

#[test]
fn all_three_flags_round_trip() {
    // R-059
    let s = Store::open_in_memory().unwrap();
    let want = AssetSnapshot {
        symbol: sym("QQQ"),
        tradable: false,
        shortable: true,
        easy_to_borrow: false,
    };
    s.put_asset_snapshot(&want, t(10)).unwrap();
    assert_eq!(s.asset_asof(&sym("QQQ"), None).unwrap(), Some(want));
}
