//! R-134: each candidate is checked for history (late start, gaps) as well as asset status.

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

const SESSIONS: [&str; 6] = [
    "2024-01-02",
    "2024-01-03",
    "2024-01-04",
    "2024-01-05",
    "2024-03-01",
    "2024-03-04",
];

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
    let at: DateTime<Utc> = "2024-04-01T00:00:00Z".parse().unwrap();
    store.upsert_bars(&rows, at).unwrap();
}

fn store() -> Store {
    let store = Store::open_in_memory().unwrap();
    let dates: Vec<NaiveDate> = SESSIONS.iter().map(|s| date(s)).collect();
    store.put_sessions(&dates).unwrap();
    store
}

fn run(mock: &MockAlpaca, store: &Store, symbols: &[&str]) -> Vec<UniverseCheck> {
    let http = loopback_client(&mock.url()).expect("loopback");
    verify_universe(store, &http, &config(symbols), date("2024-03-04")).expect("verify")
}

fn find<'a>(checks: &'a [UniverseCheck], s: &str) -> &'a UniverseCheck {
    checks
        .iter()
        .find(|c| c.symbol.as_str() == s)
        .unwrap_or_else(|| panic!("no check for {s}"))
}

// R-134: SPY (bars from the start) passes; TLT (bars from 2024-03-01, not shortable)
// fails with both `not_shortable` and `history_starts_late`.
#[test]
fn late_history_fails_with_history_starts_late_alongside_asset_reasons() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    mock.seed_asset("TLT", true, false, false);
    let store = store();
    put(&store, "SPY", &SESSIONS);
    put(&store, "TLT", &["2024-03-01", "2024-03-04"]);
    let checks = run(&mock, &store, &["SPY", "TLT"]);
    let spy = find(&checks, "SPY");
    assert!(spy.pass, "{:?}", spy.reasons);
    assert!(spy.reasons.is_empty());
    let tlt = find(&checks, "TLT");
    assert!(!tlt.pass);
    assert!(
        tlt.reasons.iter().any(|r| r == "not_shortable"),
        "{:?}",
        tlt.reasons
    );
    assert!(
        tlt.reasons.iter().any(|r| r == "history_starts_late"),
        "{:?}",
        tlt.reasons
    );
    assert!(
        !tlt.reasons.iter().any(|r| r.starts_with("gaps:")),
        "sessions before the first bar are not gaps: {:?}",
        tlt.reasons
    );
}

// R-134: a symbol with a missing session gets a reason starting `gaps:`.
#[test]
fn symbol_with_a_gap_gets_a_gaps_reason() {
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    mock.seed_asset("QQQ", true, true, true);
    let store = store();
    put(&store, "SPY", &SESSIONS);
    put(
        &store,
        "QQQ",
        &[
            "2024-01-02",
            "2024-01-03",
            "2024-01-05",
            "2024-03-01",
            "2024-03-04",
        ],
    );
    let checks = run(&mock, &store, &["SPY", "QQQ"]);
    assert!(find(&checks, "SPY").pass);
    let qqq = find(&checks, "QQQ");
    assert!(!qqq.pass);
    assert!(
        qqq.reasons.iter().any(|r| r.starts_with("gaps:")),
        "{:?}",
        qqq.reasons
    );
    assert!(
        !qqq.reasons.iter().any(|r| r == "history_starts_late"),
        "{:?}",
        qqq.reasons
    );
}
