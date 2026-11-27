//! R-443, R-126: a universe symbol with no stored bar is reported by `symbols_without_bars`, not by `check_gaps`.

use chrono::{DateTime, NaiveDate, Utc};
use pt_config::config::{parse, Config};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_data::quality::{check_gaps, symbols_without_bars};
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n\n\
[[universe]]\nsymbol = \"TLT\"\nasset_class = \"equity\"\nshortable = false\n\n\
[[universe]]\nsymbol = \"GLD\"\nasset_class = \"equity\"\nshortable = false\n";

fn date(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn config() -> Config {
    parse(UNIVERSE).expect("config parses")
}

fn store_with_calendar() -> Store {
    let store = Store::open_in_memory().unwrap();
    store
        .put_sessions(&[date("2024-01-02"), date("2024-01-03")])
        .unwrap();
    store
}

fn put(store: &Store, symbol: &str) {
    let px = Price::new(Decimal::from(100)).unwrap();
    let rows: Vec<BarRow> = [Adjustment::Raw, Adjustment::All]
        .into_iter()
        .map(|adjustment| BarRow {
            bar: Bar {
                symbol: Symbol::new(symbol).unwrap(),
                date: date("2024-01-02"),
                open: px,
                high: px,
                low: px,
                close: px,
                volume: 1000,
            },
            adjustment,
            feed: "sip".to_string(),
        })
        .collect();
    let at: DateTime<Utc> = "2024-02-01T00:00:00Z".parse().unwrap();
    store.upsert_bars(&rows, at).unwrap();
}

fn names(store: &Store) -> Vec<String> {
    symbols_without_bars(store, &config())
        .expect("symbols_without_bars succeeds")
        .iter()
        .map(|s| s.as_str().to_string())
        .collect()
}

#[test]
fn a_symbol_with_no_stored_bar_is_reported() {
    // R-443
    let store = store_with_calendar();
    put(&store, "SPY");
    put(&store, "TLT");
    assert_eq!(names(&store), vec!["GLD".to_string()]);
}

#[test]
fn one_stored_bar_is_enough_to_clear_the_report() {
    // R-443
    let store = store_with_calendar();
    for s in ["SPY", "TLT", "GLD"] {
        put(&store, s);
    }
    assert!(names(&store).is_empty());
}

#[test]
fn a_symbol_with_no_bar_adds_nothing_to_check_gaps() {
    // R-126
    let store = store_with_calendar();
    put(&store, "SPY");
    put(&store, "TLT");
    let gaps = check_gaps(&store, &config(), date("2024-01-03")).unwrap();
    assert!(gaps.iter().all(|g| g.symbol.as_str() != "GLD"));
}
