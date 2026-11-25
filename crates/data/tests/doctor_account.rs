//! R-135: the smoke test reports the account, margin and clock checks without stopping at a failure.

use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_data::doctor::{doctor, DoctorLine};
use pt_mock_alpaca::server::MockAlpaca;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn config() -> Config {
    parse(UNIVERSE).expect("config parses")
}

fn line<'a>(lines: &'a [DoctorLine], check: &str) -> &'a DoctorLine {
    lines
        .iter()
        .find(|l| l.check == check)
        .unwrap_or_else(|| panic!("no line for {check}"))
}

fn healthy() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.set_clock(
        "2025-06-24T14:15:22-04:00",
        true,
        "2025-06-25T09:30:00-04:00",
        "2025-06-24T16:00:00-04:00",
    );
    mock
}

// R-135: an active account, shorting enabled, a margin multiplier and a clock all pass.
#[test]
fn account_margin_and_clock_checks_pass_against_a_healthy_mock() {
    let mock = healthy();
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    for check in [
        "account_active",
        "shorting_enabled",
        "margin_multiplier",
        "clock",
    ] {
        let l = line(&lines, check);
        assert!(l.pass, "{check} failed: {}", l.detail);
    }
}

// R-135: each check appears once.
#[test]
fn each_account_check_is_reported_once() {
    let mock = healthy();
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    for check in [
        "account_active",
        "shorting_enabled",
        "margin_multiplier",
        "clock",
    ] {
        assert_eq!(
            lines.iter().filter(|l| l.check == check).count(),
            1,
            "{check}"
        );
    }
}

// R-135: with no account the account lines fail, and the clock line is still reported and passes.
#[test]
fn a_failing_account_does_not_stop_the_clock_check() {
    let mock = MockAlpaca::start();
    mock.set_clock(
        "2025-06-24T14:15:22-04:00",
        false,
        "2025-06-25T09:30:00-04:00",
        "2025-06-25T16:00:00-04:00",
    );
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    for check in ["account_active", "shorting_enabled", "margin_multiplier"] {
        let l = line(&lines, check);
        assert!(!l.pass, "{check} should fail without an account");
        assert!(!l.detail.is_empty(), "{check} needs a detail");
    }
    let c = line(&lines, "clock");
    assert!(c.pass, "clock failed: {}", c.detail);
}

// R-135: with no clock the clock line fails, and the account lines are still reported and pass.
#[test]
fn a_failing_clock_does_not_hide_the_account_checks() {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    let c = line(&lines, "clock");
    assert!(!c.pass);
    assert!(!c.detail.is_empty());
    for check in ["account_active", "shorting_enabled", "margin_multiplier"] {
        let l = line(&lines, check);
        assert!(l.pass, "{check} failed: {}", l.detail);
    }
}
