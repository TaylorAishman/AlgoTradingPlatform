//! K06 (R-194): `decide_with` turns signals and risk estimates into weights,
//! targets and orders.

mod common;

use common::*;
use pt_core::money::Money;
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{Positions, Side, Weights};
use pt_engine::decision::{decide_with, DecisionInputs, DecisionParams};
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::estimate::VolParams;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn signals() -> SignalSet {
    SignalSet {
        asof: d(2024, 1, 2),
        signals: BTreeMap::from([(sym("A"), 1.0), (sym("B"), -1.0)]),
        ineligible: BTreeMap::new(),
    }
}

fn risk() -> RiskEstimates {
    RiskEstimates {
        asof: d(2024, 1, 2),
        vols: BTreeMap::from([(sym("A"), 0.10), (sym("B"), 0.20)]),
        cov: BTreeMap::from([
            ((sym("A"), sym("A")), 0.01),
            ((sym("B"), sym("B")), 0.04),
            ((sym("A"), sym("B")), 0.0),
            ((sym("B"), sym("A")), 0.0),
        ]),
        ineligible: BTreeMap::new(),
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

fn params() -> DecisionParams {
    DecisionParams {
        vol: VolParams {
            com_days: 60.0,
            annualization: 261.0,
            min_obs: 126,
            max_obs: 504,
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

fn ref_prices() -> BTreeMap<pt_core::types::Symbol, pt_core::money::Price> {
    BTreeMap::from([(sym("A"), price(100)), (sym("B"), price(50))])
}

#[test]
fn worked_example_gives_weights_targets_and_orders() {
    // R-194
    let dec_out = decide_with(signals(), risk(), &inputs(), &ref_prices(), &params()).unwrap();

    let want_weights: Weights =
        BTreeMap::from([(sym("A"), dd("0.500000")), (sym("B"), dd("-0.353553"))]);
    assert_eq!(dec_out.weights, want_weights);

    let want_target: Positions = BTreeMap::from([(sym("A"), 500), (sym("B"), -707)]);
    assert_eq!(dec_out.target, want_target);

    assert_eq!(
        dec_out.orders,
        vec![
            order("B", Side::Sell, 707, 1, 50),
            order("A", Side::Buy, 500, 1, 100),
        ]
    );
    assert!(dec_out.rejected.is_empty());
}

#[test]
fn decision_carries_the_signals_and_risk_it_was_given() {
    // R-194
    let out = decide_with(signals(), risk(), &inputs(), &ref_prices(), &params()).unwrap();
    assert_eq!(out.signals, signals());
    assert_eq!(out.risk, risk());
}
