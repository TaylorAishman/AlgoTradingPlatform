//! V11 (R-375): a strategy run is compared with a benchmark run over the dates they share.
//! Each Sharpe ratio is on the run's daily returns minus BIL's, annualised; the regression
//! is on the plain daily returns.

mod common;

use std::collections::BTreeSet;

use chrono::NaiveDate;
use common::{config, date, request, store};
use pt_config::config::Config;
use pt_core::clock::FixedClock;
use pt_core::types::{Adjustment, Symbol};
use pt_stats::descriptive::returns_from_equity;
use pt_stats::ols::ols;
use pt_stats::perf::annualized_sharpe;
use pt_store::db::Store;
use pt_validate::error::ValidateError;
use pt_validate::overfit::compare_to_benchmark;
use pt_validate::runner::{run_and_record, BacktestRequest};
use rust_decimal::prelude::ToPrimitive;

const PPY: f64 = 252.0;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

fn run(store: &Store, cfg: &Config, strategy: &str, from: NaiveDate, to: NaiveDate) -> i64 {
    let req = BacktestRequest {
        strategy: strategy.into(),
        from,
        to,
        ..request()
    };
    run_and_record(store, cfg, &req, &clock())
        .expect("run")
        .run_id
}

fn equity_on(store: &Store, run_id: i64, shared: &BTreeSet<NaiveDate>) -> Vec<f64> {
    store
        .backtest_equity(run_id)
        .expect("equity")
        .iter()
        .filter(|(d, _)| shared.contains(d))
        .map(|(_, e)| e.to_f64().expect("f64"))
        .collect()
}

fn bil_on(store: &Store, cfg: &Config, shared: &BTreeSet<NaiveDate>) -> Vec<f64> {
    let first = *shared.first().expect("shared");
    let last = *shared.last().expect("shared");
    let bars = store
        .bars_asof(
            &Symbol::new("BIL").expect("symbol"),
            Adjustment::All,
            &cfg.data.feed,
            first,
            last,
            None,
        )
        .expect("bil bars");
    shared
        .iter()
        .map(|d| {
            bars.iter()
                .find(|b| b.date == *d)
                .expect("bil close")
                .close
                .value()
                .to_f64()
                .expect("f64")
        })
        .collect()
}

fn shared_dates(store: &Store, a: i64, b: i64) -> BTreeSet<NaiveDate> {
    let da: BTreeSet<NaiveDate> = store
        .backtest_equity(a)
        .expect("equity")
        .iter()
        .map(|(d, _)| *d)
        .collect();
    store
        .backtest_equity(b)
        .expect("equity")
        .iter()
        .map(|(d, _)| *d)
        .filter(|d| da.contains(d))
        .collect()
}

fn close_to(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn sharpe_ratios_are_on_excess_returns_over_the_shared_dates_and_ols_on_plain_returns() {
    // R-375: the benchmark starts later, so the shared dates are the benchmark's own
    let store = store();
    let cfg = config();
    let strat = run(&store, &cfg, "tsmom", date(2024, 3, 1), date(2024, 12, 30));
    let bench = run(
        &store,
        &cfg,
        "benchmark",
        date(2024, 6, 3),
        date(2024, 12, 30),
    );
    let shared = shared_dates(&store, strat, bench);
    assert!(shared.len() > 100);
    assert!(shared.len() < store.backtest_equity(strat).expect("equity").len());

    let report = compare_to_benchmark(&store, &cfg, strat, bench, PPY).expect("compare");

    let rs = returns_from_equity(&equity_on(&store, strat, &shared));
    let rb = returns_from_equity(&equity_on(&store, bench, &shared));
    let rf = returns_from_equity(&bil_on(&store, &cfg, &shared));
    let excess = |r: &[f64]| -> Vec<f64> { r.iter().zip(&rf).map(|(a, b)| a - b).collect() };
    close_to(report.strategy_sharpe, annualized_sharpe(&excess(&rs), PPY));
    close_to(
        report.benchmark_sharpe,
        annualized_sharpe(&excess(&rb), PPY),
    );
    let want = ols(&rs, &rb).expect("ols");
    assert_eq!(report.ols.n, want.n);
    close_to(report.ols.alpha, want.alpha);
    close_to(report.ols.beta, want.beta);
    close_to(report.ols.alpha_t, want.alpha_t);
    close_to(report.ols.r2, want.r2);
}

#[test]
fn the_sharpe_ratios_follow_periods_per_year() {
    // R-375
    let store = store();
    let cfg = config();
    let strat = run(&store, &cfg, "tsmom", date(2024, 3, 1), date(2024, 12, 30));
    let bench = run(
        &store,
        &cfg,
        "benchmark",
        date(2024, 3, 1),
        date(2024, 12, 30),
    );
    let a = compare_to_benchmark(&store, &cfg, strat, bench, 252.0).expect("252");
    let b = compare_to_benchmark(&store, &cfg, strat, bench, 63.0).expect("63");
    close_to(a.strategy_sharpe / b.strategy_sharpe, 2.0);
    close_to(a.benchmark_sharpe / b.benchmark_sharpe, 2.0);
    close_to(a.ols.beta, b.ols.beta);
}

#[test]
fn runs_with_no_common_dates_are_invalid() {
    // R-375
    let store = store();
    let cfg = config();
    let strat = run(&store, &cfg, "tsmom", date(2024, 3, 1), date(2024, 4, 30));
    let bench = run(
        &store,
        &cfg,
        "benchmark",
        date(2024, 8, 1),
        date(2024, 12, 30),
    );
    let err = compare_to_benchmark(&store, &cfg, strat, bench, PPY).expect_err("disjoint");
    assert!(matches!(err, ValidateError::Invalid(_)), "{err:?}");
}
