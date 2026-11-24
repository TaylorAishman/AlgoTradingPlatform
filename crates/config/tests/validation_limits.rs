//! C07 (R-034, R-035, R-036): `parse` validates the gross cap, the feed name
//! and that the impact term is off for the IEX feed. Failures are
//! `ConfigError::Invalid`.

use pt_config::config::{parse, Config};
use pt_config::error::ConfigError;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn try_with(extra: &str) -> Result<Config, ConfigError> {
    parse(&format!("{UNIVERSE}\n{extra}"))
}

fn invalid_message(extra: &str) -> String {
    match try_with(extra) {
        Err(ConfigError::Invalid(m)) => m,
        other => panic!("expected ConfigError::Invalid, got {other:?}"),
    }
}

const IEX_ZERO: &str = "[data]\nfeed = \"iex\"\n\
    [cost.research]\nimpact_coeff = 0\n[cost.shadow]\nimpact_coeff = 0\n";

#[test]
fn max_gross_above_one_is_invalid_and_names_the_key() {
    // R-034
    let m = invalid_message("[sizing]\nmax_gross = 1.01\n");
    assert!(m.contains("max_gross"), "message was: {m}");
}

#[test]
fn max_gross_of_one_and_a_half_are_accepted() {
    // R-034
    assert!(try_with("[sizing]\nmax_gross = 1.0\n").is_ok());
    assert!(try_with("[sizing]\nmax_gross = 0.5\n").is_ok());
}

#[test]
fn max_gross_of_zero_is_invalid() {
    // R-034
    let m = invalid_message("[sizing]\nmax_gross = 0\n");
    assert!(m.contains("max_gross"), "message was: {m}");
}

#[test]
fn default_config_is_valid() {
    // R-034, R-035: defaults (sip feed, max_gross 1) pass validation.
    assert!(try_with("").is_ok());
}

#[test]
fn sip_feed_is_accepted() {
    // R-035
    assert!(try_with("[data]\nfeed = \"sip\"\n").is_ok());
}

#[test]
fn iex_feed_with_both_impact_coeffs_zero_is_accepted() {
    // R-035
    assert!(try_with(IEX_ZERO).is_ok());
}

#[test]
fn imported_csv_feed_is_accepted() {
    // R-035
    assert!(try_with("[data]\nfeed = \"csv:mydata\"\n").is_ok());
}

#[test]
fn unknown_feed_is_invalid() {
    // R-035
    let m = invalid_message("[data]\nfeed = \"otc\"\n");
    assert!(m.contains("feed") || m.contains("otc"), "message was: {m}");
}

#[test]
fn iex_feed_with_research_impact_is_invalid() {
    // R-036
    let m = invalid_message(
        "[data]\nfeed = \"iex\"\n[cost.research]\nimpact_coeff = 1.0\n[cost.shadow]\nimpact_coeff = 0\n",
    );
    assert!(m.contains("impact_coeff"), "message was: {m}");
}

#[test]
fn iex_feed_with_default_research_impact_is_invalid() {
    // R-036: the research default impact_coeff is 1, so IEX needs it set to 0.
    let m = invalid_message("[data]\nfeed = \"iex\"\n");
    assert!(m.contains("impact_coeff"), "message was: {m}");
}

#[test]
fn iex_feed_with_shadow_impact_is_invalid() {
    // R-036: "both impact_coeff values" must be 0 for IEX.
    let m = invalid_message(
        "[data]\nfeed = \"iex\"\n[cost.research]\nimpact_coeff = 0\n[cost.shadow]\nimpact_coeff = 1.0\n",
    );
    assert!(m.contains("impact_coeff"), "message was: {m}");
}

#[test]
fn sip_feed_allows_impact() {
    // R-036: the restriction applies to IEX only.
    assert!(try_with("[data]\nfeed = \"sip\"\n[cost.research]\nimpact_coeff = 1.0\n").is_ok());
}
