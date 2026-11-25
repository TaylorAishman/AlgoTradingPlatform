//! V01 (R-367): the trial hash covers everything that changes the result and nothing else.

use chrono::NaiveDate;
use pt_config::config::{parse, Config};
use pt_validate::runner::{trial_hash, BacktestRequest};
use rust_decimal::Decimal;

const TEXT: &str = "[[universe]]\nsymbol = \"AAA\"\nasset_class = \"equity\"\nshortable = true\n\n\
[[universe]]\nsymbol = \"BBB\"\nasset_class = \"equity\"\nshortable = true\n";

const PARAMS: &str = "lookback_months=[1,3,12]";

fn cfg() -> Config {
    parse(TEXT).expect("config parses")
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

fn req() -> BacktestRequest {
    BacktestRequest {
        strategy: "tsmom".into(),
        from: date(2024, 3, 1),
        to: date(2024, 12, 30),
        initial_capital: Decimal::new(100_000, 0),
    }
}

fn base() -> String {
    trial_hash(&cfg(), &req(), PARAMS)
}

#[test]
fn hash_is_deterministic() {
    // R-367
    assert_eq!(base(), base());
    assert!(!base().is_empty());
}

#[test]
fn changing_strategy_parameters_changes_the_hash() {
    // R-367
    assert_ne!(base(), trial_hash(&cfg(), &req(), "lookback_months=[3]"));
}

#[test]
fn changing_a_universe_symbol_changes_the_hash() {
    // R-367
    let mut c = cfg();
    c.universe[1].symbol = "CCC".into();
    assert_ne!(base(), trial_hash(&c, &req(), PARAMS));
}

#[test]
fn changing_from_changes_the_hash() {
    // R-367
    let mut r = req();
    r.from = date(2024, 3, 4);
    assert_ne!(base(), trial_hash(&cfg(), &r, PARAMS));
}

#[test]
fn changing_to_changes_the_hash() {
    // R-367
    let mut r = req();
    r.to = date(2024, 12, 27);
    assert_ne!(base(), trial_hash(&cfg(), &r, PARAMS));
}

#[test]
fn changing_research_spread_changes_the_hash() {
    // R-367
    let mut c = cfg();
    c.cost.research.spread_bps += Decimal::ONE;
    assert_ne!(base(), trial_hash(&c, &req(), PARAMS));
}

#[test]
fn changing_target_vol_changes_the_hash() {
    // R-367
    let mut c = cfg();
    c.sizing.target_vol += Decimal::new(1, 2);
    assert_ne!(base(), trial_hash(&c, &req(), PARAMS));
}

#[test]
fn changing_the_data_feed_changes_the_hash() {
    // R-367
    let mut c = cfg();
    c.data.feed = if c.data.feed == "iex" { "sip" } else { "iex" }.into();
    assert_ne!(base(), trial_hash(&c, &req(), PARAMS));
}

#[test]
fn changing_output_dir_does_not_change_the_hash() {
    // R-367
    let mut c = cfg();
    c.run.output_dir = "somewhere/else".into();
    assert_eq!(base(), trial_hash(&c, &req(), PARAMS));
}

#[test]
fn changing_db_path_does_not_change_the_hash() {
    // R-367
    let mut c = cfg();
    c.run.db_path = "other.db".into();
    assert_eq!(base(), trial_hash(&c, &req(), PARAMS));
}

#[test]
fn changing_a_signoff_value_does_not_change_the_hash() {
    // R-367
    let mut c = cfg();
    c.signoff.min_sessions += 7;
    assert_eq!(base(), trial_hash(&c, &req(), PARAMS));
}
