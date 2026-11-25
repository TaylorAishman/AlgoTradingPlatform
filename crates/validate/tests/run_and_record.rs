//! V02 (R-365): a research backtest runs on adjusted prices with the research cost
//! profile and its equity is stored.

mod common;

use common::{config, date, request, sessions, store, store_with};
use pt_core::clock::FixedClock;
use pt_validate::runner::run_and_record;
use rust_decimal::Decimal;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

#[test]
fn equity_has_one_entry_per_session_in_the_range() {
    // R-365
    let out = run_and_record(&store(), &config(), &request(), &clock()).expect("run");
    let expected: Vec<_> = sessions()
        .into_iter()
        .filter(|d| *d >= date(2024, 3, 1) && *d <= date(2024, 12, 30))
        .collect();
    let got: Vec<_> = out.result.equity.iter().map(|(d, _)| *d).collect();
    assert_eq!(got, expected);
    assert_eq!(got.first(), Some(&date(2024, 3, 1)));
    assert_eq!(got.last(), Some(&date(2024, 12, 30)));
}

#[test]
fn stored_equity_equals_the_result_equity() {
    // R-365
    let store = store();
    let out = run_and_record(&store, &config(), &request(), &clock()).expect("run");
    assert_eq!(
        store.backtest_equity(out.run_id).expect("stored"),
        out.result.equity
    );
}

#[test]
fn the_outcome_carries_the_trial_hash_and_the_research_cost_total() {
    // R-365
    let out = run_and_record(&store(), &config(), &request(), &clock()).expect("run");
    assert_eq!(out.trial_hash.len(), 64);
    assert!(out.metrics.cost_total > Decimal::ZERO);
    assert_eq!(out.metrics.cost_total, out.result.cost_total);
}

#[test]
fn the_run_prices_on_adjusted_bars_not_raw_ones() {
    // R-365: raw bars that differ from the adjusted ones must not change the result.
    let plain = run_and_record(&store(), &config(), &request(), &clock()).expect("run");
    let skewed = store_with(&[], Decimal::new(3, 0));
    let other = run_and_record(&skewed, &config(), &request(), &clock()).expect("run");
    assert_eq!(plain.result.equity, other.result.equity);
}

#[test]
fn the_run_uses_the_research_costs_not_the_shadow_costs() {
    // R-365
    let base = run_and_record(&store(), &config(), &request(), &clock()).expect("run");

    let mut shadow = config();
    shadow.cost.shadow.spread_bps = Decimal::new(80, 0);
    let same = run_and_record(&store(), &shadow, &request(), &clock()).expect("run");
    assert_eq!(base.metrics.cost_total, same.metrics.cost_total);

    let mut research = config();
    research.cost.research.spread_bps = Decimal::new(80, 0);
    let more = run_and_record(&store(), &research, &request(), &clock()).expect("run");
    assert!(more.metrics.cost_total > base.metrics.cost_total);
}
