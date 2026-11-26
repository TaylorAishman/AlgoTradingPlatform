//! K27 (R-459): a decision asks the strategy for its output once.

mod common;

use common::*;
use pt_core::money::Money;
use pt_core::signal::SignalSet;
use pt_core::types::{Positions, Symbol};
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::{decide, DecisionInputs, DecisionParams};
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::VolParams;
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// Delegates to a real (fixed) strategy and counts the calls.
struct Fixed;

struct Counting {
    inner: Fixed,
    calls: Cell<u32>,
}

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

impl Strategy for Counting {
    fn id(&self) -> &str {
        self.inner.id()
    }
    fn params_canonical(&self) -> String {
        self.inner.params_canonical()
    }
    fn signals(
        &self,
        universe: &[Symbol],
        asof: chrono::NaiveDate,
        view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        self.calls.set(self.calls.get() + 1);
        self.inner.signals(universe, asof, view)
    }
}

fn view() -> MemoryView {
    let mut view = worked_view_without(&[]);
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

#[test]
fn decide_asks_the_strategy_exactly_once() {
    // R-459
    let p = DecisionParams {
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
    };
    let inp = DecisionInputs {
        signal_date: d(2024, 1, 2),
        trade_date: d(2024, 1, 3),
        ref_date: d(2024, 1, 2),
        equity: Money(dec(100000)),
        positions: Positions::new(),
        universe: vec![sym("A"), sym("B")],
        shortable: BTreeSet::from([sym("A"), sym("B")]),
    };
    let strategy = Counting {
        inner: Fixed,
        calls: Cell::new(0),
    };
    let out = decide(&inp, &strategy, &view(), &p).unwrap();
    assert!(!out.orders.is_empty(), "the decision must do real work");
    assert_eq!(strategy.calls.get(), 1);
}
