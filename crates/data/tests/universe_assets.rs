//! R-134: each candidate is checked for tradability and borrow status (asset-status reasons).

use chrono::{DateTime, NaiveDate, Utc};
use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_data::universe::{verify_universe, UniverseCheck};
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;
use pt_store::market::BarRow;
use rust_decimal::Decimal;

const SESSIONS: [&str; 4] = ["2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05"];

fn date(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn config(symbols: &[&str]) -> Config {
    let mut text = String::from("[data]\nstart = \"2024-01-02\"\n\n");
    for s in symbols {
        text.push_str(&format!(
            "[[universe]]\nsymbol = \"{s}\"\nasset_class = \"equity\"\nshortable = true\n"
        ));
    }
    parse(&text).expect("config parses")
}

fn store_with_full_history(symbols: &[&str]) -> Store {
    let store = Store::open_in_memory().unwrap();
    let dates: Vec<NaiveDate> = SESSIONS.iter().map(|s| date(s)).collect();
    store.put_sessions(&dates).unwrap();
    let px = Price::new(Decimal::from(100)).unwrap();
    let mut rows = Vec::new();
    for name in symbols {
        for adjustment in [Adjustment::Raw, Adjustment::All] {
            for d in SESSIONS {
                rows.push(BarRow {
                    bar: Bar {
                        symbol: Symbol::new(name).unwrap(),
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
    }
    let at: DateTime<Utc> = "2024-02-01T00:00:00Z".parse().unwrap();
    store.upsert_bars(&rows, at).unwrap();
    store
}

fn run(mock: &MockAlpaca, symbols: &[&str]) -> Vec<UniverseCheck> {
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = store_with_full_history(symbols);
    verify_universe(&store, &http, &config(symbols), date("2024-01-05")).expect("verify")
}

fn find<'a>(checks: &'a [UniverseCheck], s: &str) -> &'a UniverseCheck {
    checks
        .iter()
        .find(|c| c.symbol.as_str() == s)
        .unwrap_or_else(|| panic!("no check for {s}"))
}

// R-134: a tradable, shortable, easy-to-borrow symbol with full history passes with no reasons.
#[test]
fn good_symbol_passes_with_no_reasons() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    let checks = run(&mock, &["SPY"]);
    assert_eq!(checks.len(), 1);
    assert!(find(&checks, "SPY").pass);
    assert!(find(&checks, "SPY").reasons.is_empty());
}

// R-134: a symbol that is not shortable fails with `not_shortable`, others unaffected.
#[test]
fn not_shortable_symbol_fails_with_that_reason() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    mock.seed_asset("TLT", true, false, false);
    let checks = run(&mock, &["SPY", "TLT"]);
    assert_eq!(checks.len(), 2);
    assert!(find(&checks, "SPY").pass);
    let tlt = find(&checks, "TLT");
    assert!(!tlt.pass);
    assert!(
        tlt.reasons.iter().any(|r| r == "not_shortable"),
        "{:?}",
        tlt.reasons
    );
}

// R-134: a symbol that is not tradable fails with `not_tradable`.
#[test]
fn not_tradable_symbol_fails_with_that_reason() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", false, true, true);
    let checks = run(&mock, &["SPY"]);
    let spy = find(&checks, "SPY");
    assert!(!spy.pass);
    assert!(
        spy.reasons.iter().any(|r| r == "not_tradable"),
        "{:?}",
        spy.reasons
    );
}

// R-134: a shortable symbol that is hard to borrow fails with at least one reason.
#[test]
fn hard_to_borrow_symbol_fails() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, false);
    let checks = run(&mock, &["SPY"]);
    let spy = find(&checks, "SPY");
    assert!(!spy.pass);
    assert!(!spy.reasons.is_empty());
}

// R-134: a symbol unknown to the mock gets `not_found`.
#[test]
fn unknown_symbol_gets_not_found() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    let checks = run(&mock, &["SPY", "NOPE"]);
    assert!(find(&checks, "SPY").pass);
    let nope = find(&checks, "NOPE");
    assert!(!nope.pass);
    assert!(
        nope.reasons.iter().any(|r| r == "not_found"),
        "{:?}",
        nope.reasons
    );
}
