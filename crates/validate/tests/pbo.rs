//! V10 (R-374): the Probability of Backtest Overfitting is computed over stored runs on
//! the dates they all share.

mod common;

use common::{config, date, request, store};
use pt_config::config::Config;
use pt_core::clock::FixedClock;
use pt_stats::descriptive::returns_from_equity;
use pt_stats::pbo::pbo_cscv;
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_validate::error::ValidateError;
use pt_validate::overfit::pbo_for_runs;
use pt_validate::runner::{run_and_record, BacktestRequest};
use rust_decimal::prelude::ToPrimitive;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

fn config_with_lookback(m: u32) -> Config {
    let mut cfg = config();
    cfg.strategy.lookback_months = vec![m];
    cfg
}

fn record(store: &Store, m: u32, req: &BacktestRequest) -> i64 {
    run_and_record(store, &config_with_lookback(m), req, &clock())
        .expect("run")
        .run_id
}

/// The matrix `pbo_cscv` expects (`m[row][run]`): daily returns of each run's stored
/// equity over the dates all runs share.
fn common_matrix(store: &Store, run_ids: &[i64]) -> Vec<Vec<f64>> {
    let curves: Vec<Vec<(chrono::NaiveDate, f64)>> = run_ids
        .iter()
        .map(|id| {
            store
                .backtest_equity(*id)
                .expect("equity")
                .into_iter()
                .map(|(d, e)| (d, e.to_f64().expect("f64")))
                .collect()
        })
        .collect();
    let shared: Vec<chrono::NaiveDate> = curves[0]
        .iter()
        .map(|(d, _)| *d)
        .filter(|d| curves.iter().all(|c| c.iter().any(|(x, _)| x == d)))
        .collect();
    let columns: Vec<Vec<f64>> = curves
        .iter()
        .map(|c| {
            let eq: Vec<f64> = shared
                .iter()
                .map(|d| c.iter().find(|(x, _)| x == d).expect("shared date").1)
                .collect();
            returns_from_equity(&eq)
        })
        .collect();
    (0..columns[0].len())
        .map(|r| columns.iter().map(|c| c[r]).collect())
        .collect()
}

#[test]
fn three_runs_give_the_pbo_of_their_return_matrix() {
    // R-374
    let store = store();
    let ids: Vec<i64> = [1, 2, 3]
        .iter()
        .map(|m| record(&store, *m, &request()))
        .collect();
    let matrix = common_matrix(&store, &ids);
    assert!(matrix.len() >= 16);

    let got = pbo_for_runs(&store, &ids, 8).expect("pbo");
    let want = pbo_cscv(&matrix, 8).expect("pbo_cscv");
    assert_eq!(got, want);
}

#[test]
fn runs_over_different_periods_are_compared_on_the_dates_they_share() {
    // R-374
    let store = store();
    let full = request();
    let late = BacktestRequest {
        from: date(2024, 4, 1),
        ..request()
    };
    let early_end = BacktestRequest {
        to: date(2024, 11, 29),
        ..request()
    };
    let ids = vec![
        record(&store, 1, &full),
        record(&store, 2, &late),
        record(&store, 3, &early_end),
    ];
    let matrix = common_matrix(&store, &ids);
    let full_len = store.backtest_equity(ids[0]).expect("equity").len();
    assert!(matrix.len() + 1 < full_len, "the shared period is shorter");

    let got = pbo_for_runs(&store, &ids, 8).expect("pbo");
    let want = pbo_cscv(&matrix, 8).expect("pbo_cscv");
    assert_eq!(got, want);
}

#[test]
fn one_run_id_is_too_few_trials() {
    // R-374
    let store = store();
    let id = record(&store, 1, &request());
    let err = pbo_for_runs(&store, &[id], 8).expect_err("one run");
    assert!(matches!(err, ValidateError::TooFewTrials), "{err:?}");
}

#[test]
fn an_unknown_run_id_is_not_found() {
    // R-374
    let store = store();
    let id = record(&store, 1, &request());
    let err = pbo_for_runs(&store, &[id, 9_999], 8).expect_err("unknown run");
    assert!(
        matches!(err, ValidateError::Store(StoreError::NotFound(_))),
        "{err:?}"
    );
}
