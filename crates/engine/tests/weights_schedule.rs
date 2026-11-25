//! K20 (R-207): the weight schedule uses data up to each signal date only.

mod common;

use chrono::{Datelike, NaiveDate, Weekday};
use common::*;
use pt_calendar::schedule::Rebalance;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::DecisionParams;
use pt_engine::schedule::weights_schedule;
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::VolParams;
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

struct Fixed;

impl Strategy for Fixed {
    fn id(&self) -> &str {
        "fixed"
    }
    fn params_canonical(&self) -> String {
        String::new()
    }
    fn signals(
        &self,
        _universe: &[Symbol],
        asof: NaiveDate,
        _view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        Ok(SignalSet {
            asof,
            signals: BTreeMap::from([(sym("A"), 1.0), (sym("B"), 1.0)]),
            ineligible: BTreeMap::new(),
        })
    }
}

fn params() -> DecisionParams {
    let big = Decimal::from_str("100").unwrap();
    DecisionParams {
        vol: VolParams {
            com_days: 20.0,
            annualization: 261.0,
            min_obs: 10,
            max_obs: 60,
        },
        sizing: SizingParams {
            target_vol: 0.10,
            max_weight: 100.0,
            max_gross: 100.0,
        },
        limits: RiskLimits {
            max_order_notional: big * big * big,
            max_weight: big,
            max_gross: big,
            daily_loss_limit: Decimal::from_str("0.03").unwrap(),
            max_drawdown: Decimal::from_str("0.15").unwrap(),
        },
    }
}

fn schedule() -> Vec<Rebalance> {
    vec![
        Rebalance {
            signal_date: d(2024, 10, 31),
            trade_date: d(2024, 11, 1),
        },
        Rebalance {
            signal_date: d(2024, 11, 29),
            trade_date: d(2024, 12, 2),
        },
    ]
}

/// Weekday adjusted closes from 2024-08-01 to 2024-11-29; closes after `cut`
/// are scaled by `bump_after` (1.0 leaves them as they are).
fn view(cut: NaiveDate, bump_after: f64) -> MemoryView {
    let mut v = MemoryView::new();
    let mut day = d(2024, 8, 1);
    let mut i = 0u32;
    while day <= d(2024, 11, 29) {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            let a = 100.0 + f64::from(i % 7) * 1.3 + f64::from(i) * 0.1;
            let b = 50.0 + f64::from(i % 5) * 0.9 - f64::from(i % 3) * 0.4;
            let k = if day > cut { bump_after } else { 1.0 };
            // Alternate the bump so the returns, not just the level, change.
            let k = if i.is_multiple_of(2) {
                k
            } else {
                1.0 + (k - 1.0) * 2.0
            };
            v.push_adj_close(&sym("A"), day, a * k);
            v.push_adj_close(&sym("B"), day, b * k);
            i += 1;
        }
        day = day.succ_opt().unwrap();
    }
    v
}

fn run(v: &MemoryView) -> BTreeMap<NaiveDate, pt_core::types::Weights> {
    let universe = vec![sym("A"), sym("B")];
    let shortable = BTreeSet::from([sym("A"), sym("B")]);
    weights_schedule(&schedule(), &universe, &shortable, &Fixed, v, &params()).unwrap()
}

#[test]
fn schedule_is_keyed_by_trade_dates() {
    // R-207
    let w = run(&view(d(2024, 10, 31), 1.0));
    let keys: Vec<NaiveDate> = w.keys().copied().collect();
    assert_eq!(keys, vec![d(2024, 11, 1), d(2024, 12, 2)]);
}

#[test]
fn weights_are_taken_when_data_is_sufficient() {
    // R-207
    let w = run(&view(d(2024, 10, 31), 1.0));
    for entry in w.values() {
        assert!(entry.values().any(|x| !x.is_zero()));
    }
}

#[test]
fn closes_after_the_signal_date_do_not_change_its_entry() {
    // R-207
    let base = run(&view(d(2024, 10, 31), 1.0));
    let changed = run(&view(d(2024, 10, 31), 1.25));
    assert_eq!(base[&d(2024, 11, 1)], changed[&d(2024, 11, 1)]);
}

#[test]
fn closes_up_to_the_signal_date_do_change_a_later_entry() {
    // R-207: the changed data lies before the second signal date, so it must matter there.
    let base = run(&view(d(2024, 10, 31), 1.0));
    let changed = run(&view(d(2024, 10, 31), 1.25));
    assert_ne!(base[&d(2024, 12, 2)], changed[&d(2024, 12, 2)]);
}
