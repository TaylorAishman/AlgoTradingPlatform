//! K08 (R-196): `decide` reads the view and refuses to size from a missing price.

mod common;

use common::*;
use pt_core::money::Money;
use pt_core::signal::SignalSet;
use pt_core::types::{Positions, Symbol};
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::{decide, decide_with, DecisionInputs, DecisionParams};
use pt_engine::error::EngineError;
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::{estimate_risk, VolParams};
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// A strategy that always says long A and short B.
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
        asof: chrono::NaiveDate,
        _view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        Ok(SignalSet {
            asof,
            signals: BTreeMap::from([(sym("A"), 1.0), (sym("B"), -1.0)]),
            ineligible: BTreeMap::new(),
        })
    }
}

fn params() -> DecisionParams {
    DecisionParams {
        vol: VolParams {
            com_days: 60.0,
            annualization: 261.0,
            min_obs: 3,
            max_obs: 10,
        },
        sizing: SizingParams {
            target_vol: 0.10,
            max_weight: 0.5,
            max_gross: 1.0,
        },
        limits: RiskLimits {
            max_order_notional: dec(1000000),
            max_weight: dd("0.5"),
            max_gross: dec(1),
            daily_loss_limit: dd("0.03"),
            max_drawdown: dd("0.15"),
        },
    }
}

fn inputs() -> DecisionInputs {
    DecisionInputs {
        signal_date: d(2024, 1, 2),
        trade_date: d(2024, 1, 3),
        ref_date: d(2024, 1, 2),
        equity: Money(dec(100000)),
        positions: Positions::new(),
        universe: vec![sym("A"), sym("B")],
        shortable: BTreeSet::from([sym("A"), sym("B")]),
    }
}

/// Execution bars from the worked example plus adjusted closes for A and B.
fn view_with_history(skip: &[(&str, u32)], history: &[&str]) -> MemoryView {
    let mut view = worked_view_without(skip);
    let a = [100.0, 101.0, 99.0, 102.0, 100.0, 103.0];
    let b = [50.0, 50.5, 49.0, 51.0, 50.0, 50.0];
    for (i, day) in [28u32, 29, 30, 31].iter().enumerate() {
        let date = if *day >= 28 {
            d(2023, 12, *day)
        } else {
            d(2024, 1, *day)
        };
        if history.contains(&"A") {
            view.push_adj_close(&sym("A"), date, a[i]);
        }
        if history.contains(&"B") {
            view.push_adj_close(&sym("B"), date, b[i]);
        }
    }
    for (i, day) in [1u32, 2].iter().enumerate() {
        if history.contains(&"A") {
            view.push_adj_close(&sym("A"), d(2024, 1, *day), a[4 + i]);
        }
        if history.contains(&"B") {
            view.push_adj_close(&sym("B"), d(2024, 1, *day), b[4 + i]);
        }
    }
    view
}

#[test]
fn decide_matches_decide_with_on_the_views_risk_and_reference_closes() {
    // R-196: reference prices are the execution closes on ref_date
    let view = view_with_history(&[], &["A", "B"]);
    let p = params();
    let inp = inputs();
    let got = decide(&inp, &Fixed, &view, &p).unwrap();

    let risk = estimate_risk(&inp.universe, inp.signal_date, &view, &p.vol);
    assert!(risk.ineligible.is_empty(), "fixture must be eligible");
    let signals = Fixed
        .signals(&inp.universe, inp.signal_date, &view)
        .unwrap();
    let refs = BTreeMap::from([(sym("A"), price(100)), (sym("B"), price(50))]);
    let want = decide_with(signals, risk, &inp, &refs, &p).unwrap();

    assert_eq!(got, want);
    assert!(!got.orders.is_empty());
}

#[test]
fn missing_reference_bar_with_a_nonzero_weight_is_data_not_ready() {
    // R-196
    let view = view_with_history(&[("A", 2)], &["A", "B"]);
    let err = decide(&inputs(), &Fixed, &view, &params()).unwrap_err();
    match err {
        EngineError::DataNotReady { symbol, date } => {
            assert_eq!(symbol, "A");
            assert_eq!(date, d(2024, 1, 2));
        }
        other => panic!("expected DataNotReady, got {other:?}"),
    }
}

#[test]
fn missing_reference_bar_with_a_current_position_is_data_not_ready() {
    // R-196: A has no history (zero weight) but is held
    let view = view_with_history(&[("A", 2)], &["B"]);
    let mut inp = inputs();
    inp.positions = Positions::from([(sym("A"), 10)]);
    let err = decide(&inp, &Fixed, &view, &params()).unwrap_err();
    match err {
        EngineError::DataNotReady { symbol, date } => {
            assert_eq!(symbol, "A");
            assert_eq!(date, d(2024, 1, 2));
        }
        other => panic!("expected DataNotReady, got {other:?}"),
    }
}

#[test]
fn missing_bar_for_an_unheld_zero_weight_symbol_is_not_an_error() {
    // R-196: B has no history, so zero weight, and is not held
    let view = view_with_history(&[("B", 2)], &["A"]);
    let out = decide(&inputs(), &Fixed, &view, &params()).unwrap();
    assert!(out.orders.iter().all(|o| o.symbol != sym("B")));
}
