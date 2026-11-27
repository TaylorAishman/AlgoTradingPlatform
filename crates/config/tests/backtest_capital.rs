//! C16 (R-412, R-413): the `[backtest]` table holds the starting capital, and
//! the capital is not part of the configuration hash.

use std::path::{Path, PathBuf};

use pt_config::config::{load, parse, Config};
use pt_config::error::ConfigError;
use pt_config::hash::config_hash;
use rust_decimal::Decimal;

const TWO_SYMBOLS: &str = "[[universe]]\nsymbol = \"AAA\"\nasset_class = \"equity\"\nshortable = true\n\n[[universe]]\nsymbol = \"BBB\"\nasset_class = \"equity\"\nshortable = true\n";

fn with_backtest(body: &str) -> String {
    format!("{TWO_SYMBOLS}\n[backtest]\n{body}\n")
}

fn invalid_message(text: &str) -> String {
    match parse(text) {
        Err(ConfigError::Invalid(m)) => m,
        other => panic!("expected ConfigError::Invalid, got {other:?}"),
    }
}

fn cfg(text: &str) -> Config {
    parse(text).expect("config parses")
}

#[test]
fn absent_backtest_table_defaults_to_100000() {
    // R-412
    assert_eq!(
        cfg(TWO_SYMBOLS).backtest.initial_capital,
        Decimal::new(100_000, 0)
    );
}

#[test]
fn explicit_initial_capital_is_read_as_a_decimal() {
    // R-412
    let c = cfg(&with_backtest("initial_capital = 250000.5"));
    assert_eq!(c.backtest.initial_capital, Decimal::new(2_500_005, 1));
}

#[test]
fn zero_initial_capital_is_invalid_and_names_the_key() {
    // R-412
    let m = invalid_message(&with_backtest("initial_capital = 0"));
    assert!(m.contains("initial_capital"), "message was: {m}");
}

#[test]
fn negative_initial_capital_is_invalid_and_names_the_key() {
    // R-412
    let m = invalid_message(&with_backtest("initial_capital = -1"));
    assert!(m.contains("initial_capital"), "message was: {m}");
}

#[test]
fn example_config_has_initial_capital_100000() {
    // R-412
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/paper-trader.example.toml");
    let c = load(&path).expect("example config loads");
    assert_eq!(c.backtest.initial_capital, Decimal::new(100_000, 0));
}

#[test]
fn config_hash_keeps_its_value_from_before_the_backtest_table() {
    // R-413
    assert_eq!(
        config_hash(&cfg(TWO_SYMBOLS)),
        "a86a8db6c64e9ae7d8a33577985a9ac5251bcfb9102ac418da243c72a962bd83"
    );
}

#[test]
fn initial_capital_does_not_change_the_config_hash() {
    // R-413
    let with = cfg(&with_backtest("initial_capital = 250000"));
    assert_eq!(with.backtest.initial_capital, Decimal::new(250_000, 0));
    assert_eq!(config_hash(&with), config_hash(&cfg(TWO_SYMBOLS)));
}
