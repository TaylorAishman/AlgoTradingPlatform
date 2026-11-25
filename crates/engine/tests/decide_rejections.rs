//! K07 (R-195): `decide_with` records the orders the risk checks rejected.

mod common;

use common::*;
use pt_core::money::Money;
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{Positions, Side};
use pt_engine::decision::{decide_with, DecisionInputs, DecisionParams};
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::{Rejection, RiskLimits};
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
fn order_over_the_notional_limit_is_rejected_and_recorded() {
    // R-195
    let mut p = params();
    p.limits.max_order_notional = dec(40000);
    let out = decide_with(signals(), risk(), &inputs(), &ref_prices(), &p).unwrap();

    assert_eq!(out.orders, vec![order("B", Side::Sell, 707, 1, 50)]);
    assert_eq!(
        out.rejected,
        vec![(order("A", Side::Buy, 500, 1, 100), Rejection::OrderNotional)]
    );
}
