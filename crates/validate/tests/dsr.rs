//! V09 (R-373): the Deflated Sharpe Ratio of a stored run is computed from the trial
//! registry, on the run's daily excess returns over BIL.

mod common;

use common::{config, request, store, FEED};
use pt_config::config::Config;
use pt_core::clock::FixedClock;
use pt_core::types::{Adjustment, Symbol};
use pt_stats::descriptive::{kurtosis, returns_from_equity, skewness, std_sample};
use pt_stats::dsr::{deflated_sharpe, expected_max_sharpe, psr};
use pt_stats::perf::sharpe;
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_validate::error::ValidateError;
use pt_validate::overfit::dsr_for_run;
use pt_validate::runner::run_and_record;
use rust_decimal::prelude::ToPrimitive;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

fn close(d: rust_decimal::Decimal) -> f64 {
    d.to_f64().expect("f64")
}

/// The daily excess returns of a stored run: equity returns minus BIL adjusted-close
/// returns, aligned by date.
fn excess_returns(store: &Store, cfg: &Config, run_id: i64) -> Vec<f64> {
    let equity = store.backtest_equity(run_id).expect("equity");
    let first = equity.first().expect("equity").0;
    let last = equity.last().expect("equity").0;
    let bil = store
        .bars_asof(
            &Symbol::new("BIL").expect("symbol"),
            Adjustment::All,
            &cfg.data.feed,
            first,
            last,
            None,
        )
        .expect("bil bars");
    assert_eq!(cfg.data.feed, FEED);
    let eq: Vec<f64> = equity.iter().map(|(_, e)| close(*e)).collect();
    let rf: Vec<f64> = equity
        .iter()
        .map(|(d, _)| {
            close(
                bil.iter()
                    .find(|b| b.date == *d)
                    .expect("bil close on date")
                    .close
                    .value(),
            )
        })
        .collect();
    returns_from_equity(&eq)
        .iter()
        .zip(returns_from_equity(&rf))
        .map(|(r, b)| r - b)
        .collect()
}

fn close_to(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

fn config_with_lookback(m: u32) -> Config {
    let mut cfg = config();
    cfg.strategy.lookback_months = vec![m];
    cfg
}

#[test]
fn one_registered_trial_gives_psr_against_zero() {
    // R-373
    let store = store();
    let cfg = config();
    let out = run_and_record(&store, &cfg, &request(), &clock()).expect("run");
    let excess = excess_returns(&store, &cfg, out.run_id);

    let report = dsr_for_run(&store, &cfg, out.run_id, "tsmom-v1").expect("dsr");

    let sr = sharpe(&excess);
    let (skew, kurt) = (skewness(&excess), kurtosis(&excess));
    assert_eq!(report.n_trials, 1);
    assert_eq!(report.sr0, 0.0);
    assert_eq!(report.t, excess.len());
    close_to(report.sr, sr);
    close_to(report.skew, skew);
    close_to(report.kurt, kurt);
    close_to(
        report.dsr,
        psr(sr, excess.len(), skew, kurt, 0.0).expect("psr"),
    );
}

#[test]
fn three_registered_trials_deflate_by_the_variance_of_their_sharpe_ratios() {
    // R-373
    let store = store();
    let mut run_ids = Vec::new();
    for m in [1, 2, 3] {
        let out =
            run_and_record(&store, &config_with_lookback(m), &request(), &clock()).expect("run");
        run_ids.push(out.run_id);
    }
    let trials = store.trials("tsmom-v1").expect("trials");
    assert_eq!(trials.len(), 3);
    let srs: Vec<f64> = trials.iter().map(|t| t.sharpe).collect();
    let var = std_sample(&srs).powi(2);
    assert!(var > 0.0, "the three trials must differ");

    let cfg = config_with_lookback(2);
    let excess = excess_returns(&store, &cfg, run_ids[1]);
    let report = dsr_for_run(&store, &cfg, run_ids[1], "tsmom-v1").expect("dsr");

    let sr = sharpe(&excess);
    let (skew, kurt) = (skewness(&excess), kurtosis(&excess));
    assert_eq!(report.n_trials, 3);
    close_to(report.sr, sr);
    close_to(report.sr0, expected_max_sharpe(var, 3));
    assert!(report.sr0 > 0.0);
    close_to(
        report.dsr,
        deflated_sharpe(sr, excess.len(), skew, kurt, var, 3).expect("dsr"),
    );
}

#[test]
fn an_unknown_run_id_is_not_found() {
    // R-373
    let store = store();
    let cfg = config();
    run_and_record(&store, &cfg, &request(), &clock()).expect("run");
    let err = dsr_for_run(&store, &cfg, 9_999, "tsmom-v1").expect_err("unknown run");
    assert!(
        matches!(err, ValidateError::Store(StoreError::NotFound(_))),
        "{err:?}"
    );
}
