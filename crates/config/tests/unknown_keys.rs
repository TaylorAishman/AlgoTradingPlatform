//! C06 (R-032, R-033): unknown keys are rejected by name, and the config
//! offers no key for an API endpoint or an alert destination.

use pt_config::config::parse;
use pt_config::error::ConfigError;

const VALID: &str = "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn parse_error(extra: &str) -> String {
    let text = format!("{VALID}\n{extra}\n");
    match parse(&text) {
        Err(ConfigError::Parse(message)) => message,
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

#[test]
fn the_valid_base_config_parses() {
    // R-032 (guards the tests below against a broken base)
    parse(VALID).expect("the base config parses");
}

#[test]
fn unknown_key_under_run_is_rejected_by_name() {
    // R-032
    let message = parse_error("[run]\ncolour = \"red\"");
    assert!(message.contains("colour"), "{message}");
}

#[test]
fn unknown_key_in_every_section_is_rejected_by_name() {
    // R-032
    for section in [
        "data",
        "strategy",
        "stats",
        "sizing",
        "risk",
        "signoff",
        "cost.research",
        "cost.shadow",
    ] {
        let message = parse_error(&format!("[{section}]\nmystery_key = 1"));
        assert!(message.contains("mystery_key"), "{section}: {message}");
    }
}

#[test]
fn unknown_top_level_key_is_rejected_by_name() {
    // R-032
    let text = format!("mystery_top = 1\n{VALID}");
    match parse(&text) {
        Err(ConfigError::Parse(message)) => assert!(message.contains("mystery_top"), "{message}"),
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

#[test]
fn unknown_key_in_a_universe_entry_is_rejected_by_name() {
    // R-032
    let text = format!("{VALID}mystery_field = 1\n");
    match parse(&text) {
        Err(ConfigError::Parse(message)) => {
            assert!(message.contains("mystery_field"), "{message}")
        }
        other => panic!("expected ConfigError::Parse, got {other:?}"),
    }
}

#[test]
fn an_alpaca_section_with_base_url_is_rejected() {
    // R-033
    let message = parse_error("[alpaca]\nbase_url = \"https://example.com\"");
    assert!(message.contains("alpaca"), "{message}");
}

#[test]
fn an_alert_webhook_url_under_run_is_rejected() {
    // R-033
    let message = parse_error("[run]\nalert_webhook_url = \"https://example.com\"");
    assert!(message.contains("alert_webhook_url"), "{message}");
}
