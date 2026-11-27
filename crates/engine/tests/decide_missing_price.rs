//! K29 (R-457, R-458): the two missing-price cases of a decision.

mod common;

use common::*;
use pt_core::money::Money;
use pt_core::signal::SignalSet;
use pt_core::types::{Positions, Symbol};
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::{decide, DecisionInputs, DecisionParams};
use pt_engine::error::EngineError;
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::VolParams;
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// Signals only for A; B has no signal, so its weight is zero.
struct OnlyA;

impl Strategy for OnlyA {
    fn id(&self) -> &str {
        "only_a"
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
            signals: BTreeMap::from([(sym("A"), 1.0)]),
            ineligible: BTreeMap::new(),
        })
    }
}

fn view(skip: &[(&str, u32)]) -> MemoryView {
    let mut view = worked_view_without(skip);
    let a = [100.0, 101.0, 99.0, 102.0, 100.0, 103.0];
    let b = [50.0, 50.5, 49.0, 51.0, 50.0, 50.0];
    let dates = [
        d(2023, 12, 28),
        d(2023, 12, 29),
        d(2023, 12, 30),
        d(2023, 12, 31),
        d(2024, 1, 1),
        d(2024, 1, 2),
    ];
    for (i, date) in dates.iter().enumerate() {
        view.push_adj_close(&sym("A"), *date, a[i]);
        view.push_adj_close(&sym("B"), *date, b[i]);
    }
    view
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

fn inputs(positions: Positions) -> DecisionInputs {
    DecisionInputs {
        signal_date: d(2024, 1, 2),
        trade_date: d(2024, 1, 3),
        ref_date: d(2024, 1, 2),
        equity: Money(dec(100000)),
        positions,
        universe: vec![sym("A"), sym("B")],
        shortable: BTreeSet::from([sym("A"), sym("B")]),
    }
}

#[test]
fn held_position_outside_universe_without_price_is_a_pricing_error() {
    // R-457
    let mut positions = Positions::new();
    positions.insert(sym("Z"), 100);
    let r = decide(&inputs(positions), &OnlyA, &view(&[]), &params());
    let err = r.expect_err("an unpriced held position must fail the decision");
    assert!(
        !matches!(err, EngineError::DataNotReady { .. }),
        "must not be data-not-ready: {err:?}"
    );
}

#[test]
fn missing_bar_for_unweighted_unheld_symbol_does_not_stop_the_decision() {
    // R-458
    let out = decide(
        &inputs(Positions::new()),
        &OnlyA,
        &view(&[("B", 2)]),
        &params(),
    )
    .expect("B has no weight and no position");
    assert!(out.orders.iter().all(|o| o.symbol != sym("B")));
    assert!(out.rejected.iter().all(|r| r.0.symbol != sym("B")));
    assert!(!out.orders.is_empty(), "A is still traded");
}
