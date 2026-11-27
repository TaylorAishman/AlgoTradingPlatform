//! V14 (R-527): a comparison of two runs uses the dates both have and no others, and
//! `crosscheck` keeps its results after its set-up is shared with the runner.

mod common;

use common::{config, date, request, store};
use pt_core::clock::FixedClock;
use pt_validate::crosscheck::crosscheck;
use pt_validate::error::ValidateError;
use pt_validate::overfit::compare_to_benchmark;
use pt_validate::runner::{run_and_record, BacktestRequest};
use rust_decimal::Decimal;

const PPY: f64 = 252.0;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

#[test]
fn a_strategy_run_with_extra_early_dates_compares_as_one_stored_only_on_the_shared_dates() {
    // R-527
    let store = store();
    let cfg = config();
    let clock = clock();
    let strat = run_and_record(
        &store,
        &cfg,
        &BacktestRequest {
            from: date(2024, 3, 1),
            to: date(2024, 4, 30),
            ..request()
        },
        &clock,
    )
    .expect("strategy run")
    .run_id;
    let bench = run_and_record(
        &store,
        &cfg,
        &BacktestRequest {
            strategy: "benchmark".into(),
            from: date(2024, 3, 11),
            to: date(2024, 4, 30),
            ..request()
        },
        &clock,
    )
    .expect("benchmark run")
    .run_id;

    let full = store.backtest_equity(strat).expect("equity");
    let trimmed: Vec<_> = full
        .iter()
        .filter(|(d, _)| *d >= date(2024, 3, 11))
        .cloned()
        .collect();
    assert!(trimmed.len() < full.len());
    let trimmed_id = store
        .save_backtest(
            "tsmom",
            "trimmed-trial",
            "trimmed-cfg",
            &trimmed,
            common::fetched_at(),
        )
        .expect("save trimmed");

    let with_extra = compare_to_benchmark(&store, &cfg, strat, bench, PPY).expect("full");
    let only_shared = compare_to_benchmark(&store, &cfg, trimmed_id, bench, PPY).expect("trimmed");
    assert_eq!(with_extra, only_shared);
}

#[test]
fn crosscheck_zero_cost_difference_is_zero() {
    // R-527
    let store = store();
    let cfg = config();
    let r = crosscheck(&store, &cfg, &request(), true, Decimal::new(1, 6)).expect("crosscheck");
    assert!(r.equal, "{r:?}");
    assert_eq!(r.max_abs_diff, Decimal::ZERO);
}

#[test]
fn crosscheck_with_costs_passes() {
    // R-527
    let store = store();
    let cfg = config();
    let r = crosscheck(&store, &cfg, &request(), false, Decimal::new(1, 2)).expect("crosscheck");
    assert!(r.equal, "{r:?}");
}

#[test]
fn crosscheck_of_capped_orders_is_not_comparable() {
    // R-527
    let store = store();
    let cfg = config();
    let mut cfg = cfg;
    cfg.risk.max_order_notional = Decimal::new(100, 0);
    let err = crosscheck(&store, &cfg, &request(), true, Decimal::new(1, 6)).expect_err("capped");
    assert!(matches!(err, ValidateError::NotComparable(_)), "{err:?}");
}
