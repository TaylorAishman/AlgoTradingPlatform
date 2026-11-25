//! K09 (R-197): `decision_to_json` is canonical, so equal decisions give
//! byte-for-byte equal strings.

mod common;

use common::*;
use pt_core::money::Money;
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{Positions, Symbol};
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::{
    decide, decide_with, decision_to_json, Decision, DecisionInputs, DecisionParams,
};
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

fn decision_with(p: &DecisionParams) -> Decision {
    decide_with(signals(), risk(), &inputs(), &ref_prices(), p).unwrap()
}

#[test]
fn equal_inputs_give_byte_for_byte_equal_json() {
    // R-197
    let a = decision_to_json(&decision_with(&params()));
    let b = decision_to_json(&decision_with(&params()));
    assert_eq!(a, b);
}

#[test]
fn different_decisions_give_different_json() {
    // R-197
    let mut limited = params();
    limited.limits.max_order_notional = dec(40000);
    assert_ne!(
        decision_to_json(&decision_with(&params())),
        decision_to_json(&decision_with(&limited))
    );
}

#[test]
fn rejected_order_appears_in_the_json() {
    // R-197
    let mut limited = params();
    limited.limits.max_order_notional = dec(40000);
    let s = decision_to_json(&decision_with(&limited));
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(
        v["rejected"],
        serde_json::json!([{
            "order": {
                "leg": 1,
                "qty": 500,
                "reference_price": "100",
                "side": "buy",
                "symbol": "A"
            },
            "reason": "OrderNotional"
        }])
    );
}

#[test]
fn json_is_an_object_with_the_decision_fields() {
    // R-197
    let s = decision_to_json(&decision_with(&params()));
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    let obj = v.as_object().unwrap();
    for key in ["signals", "risk", "weights", "target", "orders", "rejected"] {
        assert!(obj.contains_key(key), "missing key {key} in {s}");
    }
}

#[test]
fn top_level_keys_appear_in_sorted_order() {
    // R-197: canonical form has sorted keys
    let s = decision_to_json(&decision_with(&params()));
    let keys = ["orders", "rejected", "risk", "signals", "target", "weights"];
    let positions: Vec<usize> = keys
        .iter()
        .map(|k| {
            s.find(&format!("\"{k}\":"))
                .unwrap_or_else(|| panic!("missing {k} in {s}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "keys not sorted in {s}"
    );
}

#[test]
fn weight_decimals_are_written_as_strings() {
    // R-197: decimals as strings
    let s = decision_to_json(&decision_with(&params()));
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["weights"]["A"], serde_json::json!("0.500000"));
    assert_eq!(v["weights"]["B"], serde_json::json!("-0.353553"));
}

#[test]
fn output_has_no_insignificant_whitespace_or_newlines() {
    // R-197: one canonical text, not a pretty-printed one
    let s = decision_to_json(&decision_with(&params()));
    assert!(!s.contains('\n'));
    assert!(!s.contains(": "));
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

fn view_with_history() -> MemoryView {
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
fn decide_twice_with_equal_inputs_gives_equal_json() {
    // R-197: through `decide`, not only `decide_with`
    let mut p = params();
    p.vol.min_obs = 3;
    p.vol.max_obs = 10;
    let view = view_with_history();
    let a = decide(&inputs(), &Fixed, &view, &p).unwrap();
    let b = decide(&inputs(), &Fixed, &view, &p).unwrap();
    assert!(!a.orders.is_empty());
    assert_eq!(decision_to_json(&a), decision_to_json(&b));
}
