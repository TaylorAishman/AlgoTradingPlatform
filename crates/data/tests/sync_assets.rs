//! R-133: asset status is snapshotted for every universe symbol.

use chrono::{DateTime, TimeZone, Utc};
use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_core::types::Symbol;
use pt_data::reference::sync_assets;
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;
use pt_store::market::AssetSnapshot;

fn t1() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 10, 0, 0).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn config(symbols: &[&str]) -> Config {
    let mut text = String::new();
    for s in symbols {
        text.push_str(&format!(
            "[[universe]]\nsymbol = \"{s}\"\nasset_class = \"equity\"\nshortable = true\n"
        ));
    }
    parse(&text).expect("config parses")
}

fn snap(s: &str, tradable: bool, shortable: bool, etb: bool) -> AssetSnapshot {
    AssetSnapshot {
        symbol: sym(s),
        tradable,
        shortable,
        easy_to_borrow: etb,
    }
}

// R-133: two seeded symbols yield two matching snapshots, stored, and TLT is not shortable.
#[test]
fn sync_assets_returns_and_stores_snapshots_matching_the_seeds() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    mock.seed_asset("TLT", true, false, false);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = Store::open_in_memory().expect("store");

    let snaps = sync_assets(&store, &http, &config(&["SPY", "TLT"]), t1()).expect("sync");
    assert_eq!(snaps.len(), 2);
    assert!(snaps.contains(&snap("SPY", true, true, true)));
    assert!(snaps.contains(&snap("TLT", true, false, false)));

    let tlt = store
        .asset_asof(&sym("TLT"), None)
        .unwrap()
        .expect("TLT stored");
    assert!(!tlt.shortable);
    let spy = store
        .asset_asof(&sym("SPY"), None)
        .unwrap()
        .expect("SPY stored");
    assert_eq!(spy, snap("SPY", true, true, true));
}

// R-133: a symbol the mock does not know gets a snapshot with all three flags false.
#[test]
fn unknown_symbol_gets_an_all_false_snapshot() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = Store::open_in_memory().expect("store");

    let snaps = sync_assets(&store, &http, &config(&["SPY", "QQQ"]), t1()).expect("sync");
    assert_eq!(snaps.len(), 2);
    assert!(snaps.contains(&snap("QQQ", false, false, false)));
    let stored = store
        .asset_asof(&sym("QQQ"), None)
        .unwrap()
        .expect("QQQ stored");
    assert_eq!(stored, snap("QQQ", false, false, false));
}

// R-133: the risk-free symbol (default BIL) is not queried or snapshotted.
#[test]
fn risk_free_symbol_is_not_queried() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    mock.seed_asset("BIL", true, true, true);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = Store::open_in_memory().expect("store");

    let snaps = sync_assets(&store, &http, &config(&["SPY"]), t1()).expect("sync");
    assert_eq!(snaps, vec![snap("SPY", true, true, true)]);
    assert_eq!(store.asset_asof(&sym("BIL"), None).unwrap(), None);
}
