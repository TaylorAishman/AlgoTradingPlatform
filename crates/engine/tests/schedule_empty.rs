//! K22 (R-208): a rebalance with no eligible instrument holds cash.

mod common;

use chrono::NaiveDate;
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_calendar::schedule::Rebalance;
use pt_core::cost::CostSpec;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::DecisionParams;
use pt_engine::schedule::{run_backtest, weights_schedule};
use pt_engine::simulate::SimConfig;
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::VolParams;
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

struct Wants;

impl Strategy for Wants {
    fn id(&self) -> &str {
        "wants"
    }
    fn params_canonical(&self) -> String {
        String::new()
    }
    fn signals(
        &self,
        universe: &[Symbol],
        asof: NaiveDate,
        _view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        Ok(SignalSet {
            asof,
            signals: universe.iter().map(|s| (s.clone(), 1.0)).collect(),
            ineligible: BTreeMap::new(),
        })
    }
}

fn params() -> DecisionParams {
    let frac = |s: &str| Decimal::from_str(s).unwrap();
    DecisionParams {
        vol: VolParams {
            com_days: 20.0,
            annualization: 261.0,
            min_obs: 10,
            max_obs: 60,
        },
        sizing: SizingParams {
            target_vol: 0.10,
            max_weight: 0.2,
            max_gross: 1.0,
        },
        limits: RiskLimits {
            max_order_notional: dec(1_000_000),
            max_weight: frac("0.2"),
            max_gross: dec(1),
            daily_loss_limit: frac("0.03"),
            max_drawdown: frac("0.15"),
        },
    }
}

/// Eight sessions: far fewer than `min_obs` returns for any symbol.
fn dates() -> Vec<NaiveDate> {
    (1..=8).map(|day| d(2024, 3, day + 3)).collect()
}

fn short_view(dates: &[NaiveDate]) -> MemoryView {
    let mut v = MemoryView::new();
    for s in ["A", "B"] {
        for (i, date) in dates.iter().enumerate() {
            let px = 100 + i as i64;
            v.push_exec(bar(s, *date, px, px + 1));
            v.push_adj_close(&sym(s), *date, (px + 1) as f64);
        }
    }
    v
}

fn schedule(dates: &[NaiveDate]) -> Vec<Rebalance> {
    vec![Rebalance {
        signal_date: dates[4],
        trade_date: dates[5],
    }]
}

fn universe() -> Vec<Symbol> {
    vec![sym("A"), sym("B")]
}

#[test]
fn weights_are_all_zero_when_no_symbol_is_eligible() {
    // R-208
    let dates = dates();
    let v = short_view(&dates);
    let uni = universe();
    let shortable: BTreeSet<Symbol> = uni.iter().cloned().collect();
    let w = weights_schedule(&schedule(&dates), &uni, &shortable, &Wants, &v, &params()).unwrap();
    assert_eq!(w.len(), 1);
    let entry = w.get(&dates[5]).expect("entry for the trade date");
    assert!(entry.values().all(|x| x.is_zero()), "weights: {entry:?}");
}

#[test]
fn backtest_holds_cash_with_flat_equity_and_no_fills() {
    // R-208
    let dates = dates();
    let v = short_view(&dates);
    let cal = TradingCalendar::new(dates.clone()).unwrap();
    let uni = universe();
    let shortable: BTreeSet<Symbol> = uni.iter().cloned().collect();
    let cfg = SimConfig {
        start: dates[0],
        end: dates[7],
        initial_capital: dec(1_000_000),
        cost: CostSpec::zero(),
        limits: params().limits,
        universe: uni,
    };
    let r = run_backtest(
        &cfg,
        &schedule(&dates),
        &shortable,
        &Wants,
        &v,
        &cal,
        &params(),
    )
    .unwrap();
    assert!(r.fills.is_empty());
    assert_eq!(r.equity.len(), 8);
    assert!(r.equity.iter().all(|(_, e)| *e == dec(1_000_000)));
}
