//! Shared setup for the ingest tests: a config, a calendar and a seeded mock.
#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use pt_config::config::{parse, Config};
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;

pub const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n\n\
[[universe]]\nsymbol = \"TLT\"\nasset_class = \"equity\"\nshortable = false\n";

/// The first 10 sessions of 2024.
pub const SESSIONS: [&str; 10] = [
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

pub const SYMBOLS: [&str; 3] = ["SPY", "TLT", "BIL"];

pub fn date(s: &str) -> NaiveDate {
    s.parse().expect("date")
}

pub fn instant(s: &str) -> DateTime<Utc> {
    s.parse().expect("instant")
}

pub fn config() -> Config {
    parse(UNIVERSE).expect("config parses")
}

pub fn store_with_calendar() -> Store {
    let store = Store::open_in_memory().expect("store");
    let dates: Vec<NaiveDate> = SESSIONS.iter().map(|s| date(s)).collect();
    store.put_sessions(&dates).expect("sessions");
    store
}

/// Seeds raw and adjusted bars for each symbol on each date, feed `sip`.
pub fn seed(mock: &MockAlpaca, symbols: &[&str], dates: &[&str]) {
    for adjustment in ["raw", "all"] {
        let bars = symbols
            .iter()
            .flat_map(|s| {
                dates.iter().map(move |d| {
                    (
                        s.to_string(),
                        d.to_string(),
                        100.0,
                        101.0,
                        99.0,
                        100.5,
                        1000,
                    )
                })
            })
            .collect();
        mock.seed_bars(adjustment, "sip", bars);
    }
}
