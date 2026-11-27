//! V16 (R-529): the overfitting statistics fail on a stored equity curve whose returns
//! are not finite (a zero followed by a positive value).

mod common;

use common::{config, request, store};
use pt_core::clock::FixedClock;
use pt_store::db::Store;
use pt_validate::overfit::{compare_to_benchmark, dsr_for_run, pbo_for_runs};
use pt_validate::runner::run_and_record;
use rust_decimal::Decimal;

/// Returns (good run id, bad run id): the bad run has the good run's equity with the
/// value at one date replaced by 0, so the next return divides by zero.
fn good_and_bad(store: &Store) -> (i64, i64) {
    let clock = FixedClock(common::fetched_at());
    let good = run_and_record(store, &config(), &request(), &clock)
        .expect("run")
        .run_id;
    let mut equity = store.backtest_equity(good).expect("equity");
    assert!(equity.len() > 20);
    equity[10].1 = Decimal::ZERO;
    assert!(equity[11].1 > Decimal::ZERO);
    let bad = store
        .save_backtest(
            "tsmom-v1",
            "bad-trial",
            "bad-config",
            &equity,
            common::fetched_at(),
        )
        .expect("save bad run");
    (good, bad)
}

#[test]
fn dsr_fails_on_a_run_with_a_non_finite_return() {
    // R-529
    let store = store();
    let (_, bad) = good_and_bad(&store);
    let r = dsr_for_run(&store, &config(), bad, "tsmom-v1");
    assert!(r.is_err(), "{r:?}");
}

#[test]
fn benchmark_comparison_fails_with_the_bad_run_on_either_side() {
    // R-529
    let store = store();
    let (good, bad) = good_and_bad(&store);
    let a = compare_to_benchmark(&store, &config(), bad, good, 252.0);
    assert!(a.is_err(), "bad as strategy: {a:?}");
    let b = compare_to_benchmark(&store, &config(), good, bad, 252.0);
    assert!(b.is_err(), "bad as benchmark: {b:?}");
}

#[test]
fn pbo_fails_when_one_of_two_runs_has_a_non_finite_return() {
    // R-529
    let store = store();
    let (good, bad) = good_and_bad(&store);
    let r = pbo_for_runs(&store, &[good, bad], 8);
    assert!(r.is_err(), "{r:?}");
}
