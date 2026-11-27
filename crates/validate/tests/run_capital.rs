//! V13 (R-525, R-526): a recorded run keeps its starting capital and where it came from,
//! and the same request at two capitals is one trial.

mod common;

use common::{config, request, store};
use pt_core::clock::FixedClock;
use pt_validate::runner::{run_and_record, run_and_record_with, CapitalSource};
use rust_decimal::Decimal;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

#[test]
fn a_flag_capital_is_stored_with_the_source_flag() {
    // R-525
    let store = store();
    let mut req = request();
    req.initial_capital = Decimal::new(250_000, 0);
    let out =
        run_and_record_with(&store, &config(), &req, CapitalSource::Flag, &clock()).expect("run");
    assert_eq!(
        store.backtest_capital(out.run_id).expect("read"),
        Some((Decimal::new(250_000, 0), "flag".to_string()))
    );
}

#[test]
fn run_and_record_stores_the_source_config() {
    // R-525
    let store = store();
    let req = request();
    let out = run_and_record(&store, &config(), &req, &clock()).expect("run");
    assert_eq!(
        store.backtest_capital(out.run_id).expect("read"),
        Some((req.initial_capital, "config".to_string()))
    );
}

#[test]
fn the_account_source_is_stored_as_account() {
    // R-525
    let store = store();
    let out = run_and_record_with(
        &store,
        &config(),
        &request(),
        CapitalSource::Account,
        &clock(),
    )
    .expect("run");
    let (_, source) = store
        .backtest_capital(out.run_id)
        .expect("read")
        .expect("capital stored");
    assert_eq!(source, "account");
}

#[test]
fn capital_source_names_are_flag_account_and_config() {
    // R-525
    assert_eq!(CapitalSource::Flag.as_str(), "flag");
    assert_eq!(CapitalSource::Account.as_str(), "account");
    assert_eq!(CapitalSource::Config.as_str(), "config");
}

#[test]
fn two_capitals_of_one_request_are_one_trial_with_two_runs() {
    // R-526
    let store = store();
    let mut low = request();
    low.initial_capital = Decimal::new(100_000, 0);
    let mut high = request();
    high.initial_capital = Decimal::new(250_000, 0);
    let a =
        run_and_record_with(&store, &config(), &low, CapitalSource::Flag, &clock()).expect("run");
    let b =
        run_and_record_with(&store, &config(), &high, CapitalSource::Flag, &clock()).expect("run");
    assert_eq!(a.trial_hash, b.trial_hash);
    let trials = store.trials("tsmom-v1").expect("trials");
    assert_eq!(trials.len(), 1);
    assert_eq!(trials[0].runs, 2);
}
