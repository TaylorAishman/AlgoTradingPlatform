//! R-161: each weight is capped at `max_weight` in absolute value.

use pt_core::signal::RiskEstimates;
use pt_core::types::{Symbol, Weights};
use pt_portfolio::weights::{target_weights, SizingParams};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// A has vol 0.1 and B has vol 0.2, uncorrelated, so u = (10, -5), u'Cu = 2.
fn risk() -> RiskEstimates {
    let (a, b) = (sym("A"), sym("B"));
    let mut vols = BTreeMap::new();
    vols.insert(a.clone(), 0.1);
    vols.insert(b.clone(), 0.2);
    let mut cov = BTreeMap::new();
    cov.insert((a.clone(), a.clone()), 0.01);
    cov.insert((b.clone(), b.clone()), 0.04);
    cov.insert((a.clone(), b.clone()), 0.0);
    cov.insert((b, a), 0.0);
    RiskEstimates {
        asof: Default::default(),
        vols,
        cov,
        ineligible: BTreeMap::new(),
    }
}

fn run(signal_a: f64, signal_b: f64, max_weight: f64) -> Weights {
    let mut signals = BTreeMap::new();
    signals.insert(sym("A"), signal_a);
    signals.insert(sym("B"), signal_b);
    let shortable: BTreeSet<Symbol> = [sym("A"), sym("B")].into_iter().collect();
    let p = SizingParams {
        target_vol: 0.1,
        max_weight,
        max_gross: 2.0,
    };
    target_weights(&signals, &risk(), &shortable, &p)
}

#[test]
fn long_weight_above_the_cap_is_clamped() {
    // R-161
    let w = run(1.0, -1.0, 0.5);

    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], dec("0.500000"));
    assert_eq!(w[&sym("B")], dec("-0.353553"));
}

#[test]
fn short_weight_beyond_the_cap_is_clamped_to_minus_cap() {
    // R-161: the cap applies to the absolute value.
    let w = run(1.0, -1.0, 0.3);

    assert_eq!(w[&sym("A")], dec("0.300000"));
    assert_eq!(w[&sym("B")], dec("-0.300000"));
}

#[test]
fn weights_within_the_cap_are_unchanged() {
    // R-161
    let w = run(1.0, -1.0, 0.75);

    assert_eq!(w[&sym("A")], dec("0.707107"));
    assert_eq!(w[&sym("B")], dec("-0.353553"));
}
