//! C17 (R-414): each sign-off minimum must be at least 1.

use pt_config::config::parse;
use pt_config::error::ConfigError;

const TWO_SYMBOLS: &str = "[[universe]]\nsymbol = \"AAA\"\nasset_class = \"equity\"\nshortable = true\n\n[[universe]]\nsymbol = \"BBB\"\nasset_class = \"equity\"\nshortable = true\n";

fn with_signoff(body: &str) -> String {
    format!("{TWO_SYMBOLS}\n[signoff]\n{body}\n")
}

fn assert_refused_naming(key: &str) {
    let text = with_signoff(&format!("{key} = 0"));
    match parse(&text) {
        Err(ConfigError::Invalid(m)) => {
            assert!(m.contains(key), "message was: {m}");
        }
        other => panic!("expected ConfigError::Invalid for {key} = 0, got {other:?}"),
    }
}

#[test]
fn zero_min_sessions_is_invalid_and_names_the_key() {
    // R-414
    assert_refused_naming("min_sessions");
}

#[test]
fn zero_min_rebalances_is_invalid_and_names_the_key() {
    // R-414
    assert_refused_naming("min_rebalances");
}

#[test]
fn zero_min_fills_is_invalid_and_names_the_key() {
    // R-414
    assert_refused_naming("min_fills");
}

#[test]
fn minimums_of_one_are_accepted() {
    // R-414
    let c = parse(&with_signoff(
        "min_sessions = 1\nmin_rebalances = 1\nmin_fills = 1",
    ))
    .expect("minimums of 1 are accepted");
    assert_eq!(c.signoff.min_sessions, 1);
    assert_eq!(c.signoff.min_rebalances, 1);
    assert_eq!(c.signoff.min_fills, 1);
}
