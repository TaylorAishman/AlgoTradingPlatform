//! R-446: the margin check of `doctor` passes at a multiplier of exactly 2 and fails below it.

use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_data::doctor::{doctor, DoctorLine};
use pt_mock_alpaca::server::MockAlpaca;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn config() -> Config {
    parse(UNIVERSE).expect("config parses")
}

fn margin_line(multiplier: Option<&str>) -> DoctorLine {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    if let Some(m) = multiplier {
        mock.seed_account_multiplier(m);
    }
    let http = loopback_client(&mock.url()).expect("loopback");
    doctor(&http, &config())
        .into_iter()
        .find(|l| l.check == "margin_multiplier")
        .expect("margin_multiplier line")
}

// R-446: the default multiplier of 2 passes.
#[test]
fn margin_check_passes_at_the_default_multiplier_of_exactly_2() {
    let l = margin_line(None);
    assert!(l.pass, "default multiplier failed: {}", l.detail);
}

// R-446: an explicit multiplier of 2 passes.
#[test]
fn margin_check_passes_at_an_explicit_multiplier_of_2() {
    let l = margin_line(Some("2"));
    assert!(l.pass, "multiplier 2 failed: {}", l.detail);
}

// R-446: a multiplier of 4 passes.
#[test]
fn margin_check_passes_at_a_multiplier_of_4() {
    let l = margin_line(Some("4"));
    assert!(l.pass, "multiplier 4 failed: {}", l.detail);
}

// R-446: a multiplier of 1 fails, with a detail.
#[test]
fn margin_check_fails_at_a_multiplier_of_1() {
    let l = margin_line(Some("1"));
    assert!(!l.pass, "multiplier 1 should fail");
    assert!(!l.detail.is_empty());
}

// R-446: a multiplier just below 2 fails.
#[test]
fn margin_check_fails_just_below_2() {
    let l = margin_line(Some("1.99"));
    assert!(!l.pass, "multiplier 1.99 should fail");
}
