//! V04 (R-365, R-366): a research backtest is registered as a trial; a repeat is a run,
//! a changed parameter is a new trial.

mod common;

use common::{config, date, request, store, FEED};
use pt_core::clock::FixedClock;
use pt_core::types::{Adjustment, Symbol};
use pt_stats::descriptive::returns_from_equity;
use pt_stats::perf::sharpe;
use pt_validate::runner::run_and_record;
use rust_decimal::prelude::ToPrimitive;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

#[test]
fn a_run_registers_one_trial_with_its_hash_and_per_period_sharpe() {
    // R-365
    let store = store();
    let out = run_and_record(&store, &config(), &request(), &clock()).expect("run");

    let trials = store.trials("tsmom-v1").expect("trials");
    assert_eq!(trials.len(), 1);
    let t = &trials[0];
    assert_eq!(t.trial_hash, out.trial_hash);
    assert_eq!(t.runs, 1);

    let equity: Vec<f64> = out
        .result
        .equity
        .iter()
        .map(|(_, e)| e.to_f64().expect("f64"))
        .collect();
    let rets = returns_from_equity(&equity);
    assert_eq!(t.n_obs as usize, rets.len());

    let bil = Symbol::new("BIL").expect("symbol");
    let closes: Vec<f64> = store
        .bars_asof(
            &bil,
            Adjustment::All,
            FEED,
            date(2024, 3, 1),
            date(2024, 12, 30),
            None,
        )
        .expect("bil bars")
        .iter()
        .map(|b| b.close.value().to_f64().expect("f64"))
        .collect();
    let excess: Vec<f64> = rets
        .iter()
        .zip(returns_from_equity(&closes))
        .map(|(r, b)| r - b)
        .collect();
    assert!((t.sharpe - sharpe(&excess)).abs() <= 1e-9);
}

#[test]
fn repeating_a_backtest_counts_a_run_not_a_trial() {
    // R-366
    let store = store();
    run_and_record(&store, &config(), &request(), &clock()).expect("first");
    run_and_record(&store, &config(), &request(), &clock()).expect("second");

    let trials = store.trials("tsmom-v1").expect("trials");
    assert_eq!(trials.len(), 1);
    assert_eq!(trials[0].runs, 2);
}

#[test]
fn a_repeat_stores_a_second_backtest() {
    // R-366
    let store = store();
    let a = run_and_record(&store, &config(), &request(), &clock()).expect("first");
    let b = run_and_record(&store, &config(), &request(), &clock()).expect("second");
    assert_ne!(a.run_id, b.run_id);
    assert_eq!(
        store.backtest_equity(a.run_id).expect("a"),
        store.backtest_equity(b.run_id).expect("b")
    );
}

#[test]
fn changing_a_parameter_adds_a_trial() {
    // R-366
    let store = store();
    let first = run_and_record(&store, &config(), &request(), &clock()).expect("first");
    let mut cfg = config();
    cfg.strategy.lookback_months = vec![3];
    let second = run_and_record(&store, &cfg, &request(), &clock()).expect("second");

    assert_ne!(first.trial_hash, second.trial_hash);
    let trials = store.trials("tsmom-v1").expect("trials");
    assert_eq!(trials.len(), 2);
    assert!(trials.iter().all(|t| t.runs == 1));
}

#[test]
fn a_benchmark_registers_under_its_own_family() {
    // R-366
    let store = store();
    run_and_record(&store, &config(), &request(), &clock()).expect("tsmom");
    let mut req = request();
    req.strategy = "benchmark".into();
    let out = run_and_record(&store, &config(), &req, &clock()).expect("benchmark");

    let bench = store.trials("bh-volscaled-v1").expect("bench trials");
    assert_eq!(bench.len(), 1);
    assert_eq!(bench[0].trial_hash, out.trial_hash);
    assert_eq!(store.trials("tsmom-v1").expect("tsmom trials").len(), 1);
}
