//! V03 (R-368): the reported metrics are the statistics of the stored equity.

mod common;

use common::{config, request, store, FEED};
use pt_core::clock::FixedClock;
use pt_core::types::{Adjustment, Symbol};
use pt_stats::descriptive::returns_from_equity;
use pt_stats::perf::{annualized_sharpe, annualized_vol, max_drawdown};
use pt_validate::runner::{run_and_record, BacktestOutcome};
use rust_decimal::prelude::ToPrimitive;

const TOL: f64 = 1e-9;

fn outcome() -> (pt_store::db::Store, BacktestOutcome) {
    let store = store();
    let clock = FixedClock(common::fetched_at());
    let out = run_and_record(&store, &config(), &request(), &clock).expect("run");
    (store, out)
}

fn equity(out: &BacktestOutcome) -> Vec<f64> {
    out.result
        .equity
        .iter()
        .map(|(_, e)| e.to_f64().expect("f64"))
        .collect()
}

fn close_to(got: f64, want: f64) {
    assert!(
        (got - want).abs() <= TOL,
        "got {got}, want {want} (tolerance {TOL})"
    );
}

#[test]
fn volatility_is_the_annualized_vol_of_daily_equity_returns() {
    // R-368
    let (_, out) = outcome();
    let rets = returns_from_equity(&equity(&out));
    assert!(out.metrics.vol > 0.0, "the fixture run has varying equity");
    close_to(out.metrics.vol, annualized_vol(&rets, 252.0));
}

#[test]
fn max_drawdown_is_that_of_the_stored_equity() {
    // R-368
    let (_, out) = outcome();
    let want = max_drawdown(&equity(&out));
    assert!(want > 0.0, "the fixture run has a drawdown");
    close_to(out.metrics.max_drawdown, want);
}

#[test]
fn sharpe_is_of_equity_returns_minus_bil_adjusted_returns() {
    // R-368
    let (store, out) = outcome();
    let req = request();
    let bil = Symbol::new("BIL").expect("symbol");
    let bars = store
        .bars_asof(&bil, Adjustment::All, FEED, req.from, req.to, None)
        .expect("bil bars");
    let closes: Vec<f64> = bars
        .iter()
        .map(|b| b.close.value().to_f64().expect("f64"))
        .collect();
    let bil_rets = returns_from_equity(&closes);
    let rets = returns_from_equity(&equity(&out));
    assert_eq!(bil_rets.len(), rets.len());
    let excess: Vec<f64> = rets.iter().zip(&bil_rets).map(|(r, b)| r - b).collect();
    let want = annualized_sharpe(&excess, 252.0);
    assert!(want != 0.0, "the fixture run has a nonzero Sharpe ratio");
    close_to(out.metrics.sharpe, want);
}

#[test]
fn turnover_is_traded_notional_over_mean_equity_per_year() {
    // R-368
    let (_, out) = outcome();
    let eq = equity(&out);
    let mean_eq = eq.iter().sum::<f64>() / eq.len() as f64;
    let n_returns = (eq.len() - 1) as f64;
    let notional = out.result.traded_notional.to_f64().expect("f64");
    assert!(notional > 0.0, "the fixture run trades");
    close_to(
        out.metrics.turnover,
        notional / mean_eq / (n_returns / 252.0),
    );
}

#[test]
fn a_missing_risk_free_bar_is_an_error_not_a_shifted_sharpe() {
    // R-368
    let skipped = common::date(2024, 6, 12);
    let store = common::store_with(&[("BIL", skipped)], rust_decimal::Decimal::ONE);
    let clock = FixedClock(common::fetched_at());
    let err = run_and_record(&store, &config(), &request(), &clock)
        .expect_err("a missing BIL bar must fail the run");
    let msg = err.to_string();
    assert!(msg.contains("BIL"), "{msg}");
    assert!(msg.contains("2024-06-12"), "{msg}");
}
