//! R-043: a secret must never print.

use pt_config::secrets::{ApiKeys, Secret};

fn keys() -> ApiKeys {
    ApiKeys {
        key_id: Secret::new("K1ID".to_string()),
        secret_key: Secret::new("S1SECRET".to_string()),
    }
}

// R-043
#[test]
fn debug_of_api_keys_hides_the_secret_and_shows_stars() {
    let out = format!("{:?}", keys());
    assert!(!out.contains("S1SECRET"), "leaked: {out}");
    assert!(!out.contains("K1ID"), "leaked: {out}");
    assert!(out.contains("***"), "no redaction marker: {out}");
}

// R-043
#[test]
fn display_of_a_secret_hides_the_value_and_shows_stars() {
    let out = format!("{}", keys().secret_key);
    assert!(!out.contains("S1SECRET"), "leaked: {out}");
    assert!(out.contains("***"), "no redaction marker: {out}");
}

// R-043
#[test]
fn debug_of_a_secret_hides_the_value_and_shows_stars() {
    let out = format!("{:?}", keys().secret_key);
    assert!(!out.contains("S1SECRET"), "leaked: {out}");
    assert!(out.contains("***"), "no redaction marker: {out}");
}

// R-043: redaction must not stop the caller reading the value on purpose.
#[test]
fn expose_returns_the_underlying_value() {
    let k = keys();
    assert_eq!(k.secret_key.expose(), "S1SECRET");
    assert_eq!(k.key_id.expose(), "K1ID");
}
