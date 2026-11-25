//! R-135: the smoke test probes the data feed and the asset fields, and a denied feed does not hide the other lines.

use pt_alpaca::http::loopback_client;
use pt_config::config::{parse, Config};
use pt_data::doctor::{doctor, DoctorLine};
use pt_mock_alpaca::server::MockAlpaca;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

const ALL_CHECKS: [&str; 6] = [
    "account_active",
    "shorting_enabled",
    "margin_multiplier",
    "clock",
    "feed",
    "asset_fields",
];

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
    mock.seed_asset("SPY", true, true, true);
    for adjustment in ["raw", "all"] {
        let bars = ["2024-01-02", "2025-06-18", "2025-06-20", "2025-06-23"]
            .iter()
            .map(|d| {
                (
                    "SPY".to_string(),
                    d.to_string(),
                    100.0,
                    101.0,
                    99.0,
                    100.5,
                    1000,
                )
            })
            .collect();
        mock.seed_bars(adjustment, "sip", bars);
    }
    mock
}

// R-135: against a fully seeded mock every line is present and passes.
#[test]
fn every_check_passes_against_a_healthy_mock() {
    let mock = healthy();
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    for check in ALL_CHECKS {
        assert_eq!(
            lines.iter().filter(|l| l.check == check).count(),
            1,
            "{check} should appear once"
        );
        let l = line(&lines, check);
        assert!(l.pass, "{check} failed: {}", l.detail);
    }
}

// R-135: a denied feed fails the feed line, names iex, and leaves the other lines in place.
#[test]
fn a_denied_feed_fails_the_feed_line_and_mentions_iex() {
    let mock = healthy();
    mock.deny_feed("sip");
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    let feed = line(&lines, "feed");
    assert!(!feed.pass, "feed should fail when sip is denied");
    assert!(
        feed.detail.contains("iex"),
        "detail should suggest iex: {}",
        feed.detail
    );
    for check in ALL_CHECKS.into_iter().filter(|c| *c != "feed") {
        let l = line(&lines, check);
        assert!(l.pass, "{check} failed: {}", l.detail);
    }
}

// R-135: an asset the mock does not know fails the asset line, and the feed line is still reported.
#[test]
fn an_unknown_asset_fails_the_asset_line_only() {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.set_clock(
        "2025-06-24T14:15:22-04:00",
        true,
        "2025-06-25T09:30:00-04:00",
        "2025-06-24T16:00:00-04:00",
    );
    mock.seed_bars(
        "raw",
        "sip",
        vec![(
            "SPY".to_string(),
            "2025-06-23".to_string(),
            100.0,
            101.0,
            99.0,
            100.5,
            1000,
        )],
    );
    let http = loopback_client(&mock.url()).expect("loopback");
    let lines = doctor(&http, &config());
    let a = line(&lines, "asset_fields");
    assert!(!a.pass, "asset_fields should fail for an unknown symbol");
    assert!(!a.detail.is_empty());
    for check in ALL_CHECKS {
        line(&lines, check);
    }
    let c = line(&lines, "clock");
    assert!(c.pass, "clock failed: {}", c.detail);
}
