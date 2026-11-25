//! R-126: missing sessions are reported per symbol, and sessions before a symbol's first bar are not gaps.

use chrono::{DateTime, NaiveDate, Utc};
use pt_config::config::{parse, Config};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_data::quality::check_gaps;
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n\n\
[[universe]]\nsymbol = \"TLT\"\nasset_class = \"equity\"\nshortable = false\n";

const SESSIONS: [&str; 10] = [
    "2024-01-02",
    "2024-01-03",
    "2024-01-04",
    "2024-01-05",
    "2024-01-08",
    "2024-01-09",
    "2024-01-10",
    "2024-01-11",
    "2024-01-12",
    "2024-01-16",
];

fn date(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn config() -> Config {
    parse(UNIVERSE).expect("config parses")
}

fn store_with_calendar() -> Store {
    let store = Store::open_in_memory().unwrap();
    let dates: Vec<NaiveDate> = SESSIONS.iter().map(|s| date(s)).collect();
    store.put_sessions(&dates).unwrap();
    store
}

fn put(store: &Store, symbol: &str, dates: &[&str]) {
    let px = Price::new(Decimal::from(100)).unwrap();
    let mut rows = Vec::new();
    for adjustment in [Adjustment::Raw, Adjustment::All] {
        for d in dates {
            rows.push(BarRow {
                bar: Bar {
                    symbol: Symbol::new(symbol).unwrap(),
                    date: date(d),
                    open: px,
                    high: px,
                    low: px,
                    close: px,
                    volume: 1000,
                },
                adjustment,
                feed: "sip".to_string(),
            });
        }
    }
    let at: DateTime<Utc> = "2024-02-01T00:00:00Z".parse().unwrap();
    store.upsert_bars(&rows, at).unwrap();
}

fn gaps(store: &Store, through: &str) -> Vec<(String, NaiveDate)> {
    let mut out: Vec<(String, NaiveDate)> = check_gaps(store, &config(), date(through))
        .expect("check_gaps succeeds")
        .into_iter()
        .map(|g| (g.symbol.as_str().to_string(), g.date))
        .collect();
    out.sort();
    out
}

#[test]
fn one_missing_session_is_reported_for_its_symbol_only() {
    // R-126
    let store = store_with_calendar();
    let without: Vec<&str> = SESSIONS
        .iter()
        .copied()
        .filter(|s| *s != "2024-01-09")
        .collect();
    put(&store, "SPY", &without);
    put(&store, "TLT", &SESSIONS);
    put(&store, "BIL", &SESSIONS);
    assert_eq!(
        gaps(&store, "2024-01-16"),
        vec![("SPY".to_string(), date("2024-01-09"))]
    );
}

#[test]
fn complete_history_has_no_gaps() {
    // R-126
    let store = store_with_calendar();
    for s in ["SPY", "TLT", "BIL"] {
        put(&store, s, &SESSIONS);
    }
    assert!(gaps(&store, "2024-01-16").is_empty());
}

#[test]
fn sessions_before_the_first_stored_bar_are_not_gaps() {
    // R-126
    let store = store_with_calendar();
    put(&store, "SPY", &SESSIONS);
    put(&store, "TLT", &SESSIONS[4..]);
    put(&store, "BIL", &SESSIONS);
    assert!(gaps(&store, "2024-01-16").is_empty());
}

#[test]
fn a_missing_session_after_the_first_bar_is_a_gap_even_when_early_ones_are_absent() {
    // R-126
    let store = store_with_calendar();
    put(&store, "SPY", &SESSIONS);
    put(&store, "BIL", &SESSIONS);
    let tlt: Vec<&str> = SESSIONS[2..]
        .iter()
        .copied()
        .filter(|s| *s != "2024-01-11")
        .collect();
    put(&store, "TLT", &tlt);
    assert_eq!(
        gaps(&store, "2024-01-16"),
        vec![("TLT".to_string(), date("2024-01-11"))]
    );
}

#[test]
fn sessions_after_the_through_date_are_not_checked() {
    // R-126
    let store = store_with_calendar();
    for s in ["SPY", "TLT", "BIL"] {
        put(&store, s, &SESSIONS[..8]);
    }
    assert!(gaps(&store, "2024-01-11").is_empty());
}
