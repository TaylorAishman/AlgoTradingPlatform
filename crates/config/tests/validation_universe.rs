//! C08 (R-037): `parse` validates the universe. It must be non-empty, free of
//! duplicate symbols and must not contain the risk-free proxy. Failures are
//! `ConfigError::Invalid`; an unknown asset class stays a `Parse` error.

use pt_config::config::{parse, Config};
use pt_config::error::ConfigError;

fn entry(symbol: &str, class: &str) -> String {
    format!("[[universe]]\nsymbol = \"{symbol}\"\nasset_class = \"{class}\"\nshortable = true\n")
}

fn invalid_message(text: &str) -> String {
    match parse(text) {
        Err(ConfigError::Invalid(m)) => m,
        other => panic!("expected ConfigError::Invalid, got {other:?}"),
    }
}

fn valid(text: &str) -> Config {
    parse(text).expect("config is valid")
}

#[test]
fn a_config_with_no_universe_entry_is_invalid() {
    // R-037
    invalid_message("");
}

#[test]
fn an_empty_universe_array_is_invalid() {
    // R-037
    invalid_message("universe = []\n");
}

#[test]
fn a_config_with_other_sections_but_no_universe_is_invalid() {
    // R-037
    invalid_message("[sizing]\nmax_gross = 0.5\n");
}

#[test]
fn a_duplicate_symbol_is_invalid_and_named() {
    // R-037
    let text = format!(
        "{}{}{}",
        entry("SPY", "equity"),
        entry("TLT", "treasury"),
        entry("SPY", "equity")
    );
    let m = invalid_message(&text);
    assert!(m.contains("SPY"), "message was: {m}");
}

#[test]
fn a_duplicate_with_a_different_class_is_still_invalid() {
    // R-037
    let text = format!("{}{}", entry("GLD", "commodity"), entry("GLD", "equity"));
    let m = invalid_message(&text);
    assert!(m.contains("GLD"), "message was: {m}");
}

#[test]
fn the_default_risk_free_symbol_in_the_universe_is_invalid() {
    // R-037
    let text = format!("{}{}", entry("SPY", "equity"), entry("BIL", "treasury"));
    let m = invalid_message(&text);
    assert!(m.contains("BIL"), "message was: {m}");
}

#[test]
fn a_configured_risk_free_symbol_in_the_universe_is_invalid() {
    // R-037
    let text = format!(
        "[strategy]\nrisk_free_symbol = \"SHY\"\n{}{}",
        entry("SPY", "equity"),
        entry("SHY", "treasury")
    );
    let m = invalid_message(&text);
    assert!(m.contains("SHY"), "message was: {m}");
}

#[test]
fn bil_is_allowed_when_it_is_not_the_risk_free_symbol() {
    // R-037: the ban is on the configured proxy, not on the name BIL
    let text = format!(
        "[strategy]\nrisk_free_symbol = \"SHY\"\n{}{}",
        entry("SPY", "equity"),
        entry("BIL", "treasury")
    );
    assert_eq!(valid(&text).universe.len(), 2);
}

#[test]
fn a_distinct_universe_without_the_proxy_is_valid() {
    // R-037
    let text = format!(
        "{}{}{}",
        entry("SPY", "equity"),
        entry("TLT", "treasury"),
        entry("GLD", "commodity")
    );
    assert_eq!(valid(&text).universe.len(), 3);
}

#[test]
fn an_unknown_asset_class_is_a_parse_error_not_invalid() {
    // R-037
    let err = parse(&entry("BTC", "crypto")).expect_err("crypto is not a class");
    assert!(matches!(err, ConfigError::Parse(_)), "{err:?}");
}
