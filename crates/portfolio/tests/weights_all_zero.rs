//! R-452: zero signals, or risk that cannot be measured, give weight 0 for every symbol.

use pt_core::signal::RiskEstimates;
use pt_core::types::{Symbol, Weights};
use pt_portfolio::weights::{target_weights, SizingParams};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn risk(cov_value: f64) -> RiskEstimates {
    let (a, b) = (sym("A"), sym("B"));
    let vols: BTreeMap<Symbol, f64> = [(a.clone(), 0.1), (b.clone(), 0.2)].into_iter().collect();
    let mut cov = BTreeMap::new();
    for x in [&a, &b] {
        for y in [&a, &b] {
            cov.insert((x.clone(), y.clone()), cov_value);
        }
    }
    RiskEstimates {
        asof: Default::default(),
        vols,
        cov,
        ineligible: BTreeMap::new(),
    }
}

fn run(signal_a: f64, signal_b: f64, risk: &RiskEstimates) -> Weights {
    let signals: BTreeMap<Symbol, f64> = [(sym("A"), signal_a), (sym("B"), signal_b)]
        .into_iter()
        .collect();
    let shortable: BTreeSet<Symbol> = [sym("A"), sym("B")].into_iter().collect();
    let p = SizingParams {
        target_vol: 0.05,
        max_weight: 1.0,
        max_gross: 2.0,
    };
    target_weights(&signals, risk, &shortable, &p)
}

fn assert_all_zero(w: &Weights) {
    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], Decimal::ZERO);
    assert_eq!(w[&sym("B")], Decimal::ZERO);
}

#[test]
fn every_signal_zero_gives_zero_for_every_symbol() {
    // R-452
    assert_all_zero(&run(0.0, 0.0, &risk(0.01)));
}

#[test]
fn all_zero_covariance_with_nonzero_signals_gives_zero_for_every_symbol() {
    // R-452
    assert_all_zero(&run(1.0, -1.0, &risk(0.0)));
}

#[test]
fn positive_covariance_with_the_same_signals_is_not_all_zero() {
    // R-452: guards against a stub that always returns zeros.
    let w = run(1.0, 1.0, &risk(0.01));
    assert_eq!(w.len(), 2);
    assert!(w[&sym("A")] > Decimal::ZERO);
    assert!(w[&sym("B")] > Decimal::ZERO);
}
