//! R-164: weights are zero, not undefined, when there is no signal or a symbol is ineligible.

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

/// A has vol 0.1 and B has vol 0.2, uncorrelated.
fn risk(ineligible: &[&str], with_b_vol: bool) -> RiskEstimates {
    let (a, b) = (sym("A"), sym("B"));
    let mut vols = BTreeMap::new();
    vols.insert(a.clone(), 0.1);
    if with_b_vol {
        vols.insert(b.clone(), 0.2);
    }
    let mut cov = BTreeMap::new();
    cov.insert((a.clone(), a.clone()), 0.01);
    cov.insert((b.clone(), b.clone()), 0.04);
    cov.insert((a.clone(), b.clone()), 0.0);
    cov.insert((b, a), 0.0);
    RiskEstimates {
        asof: Default::default(),
        vols,
        cov,
        ineligible: ineligible
            .iter()
            .map(|s| (sym(s), "ineligible".to_string()))
            .collect(),
    }
}

fn run(signal_a: f64, signal_b: f64, risk: &RiskEstimates) -> Weights {
    let mut signals = BTreeMap::new();
    signals.insert(sym("A"), signal_a);
    signals.insert(sym("B"), signal_b);
    let shortable: BTreeSet<Symbol> = [sym("A"), sym("B")].into_iter().collect();
    let p = SizingParams {
        target_vol: 0.05,
        max_weight: 1.0,
        max_gross: 2.0,
    };
    target_weights(&signals, risk, &shortable, &p)
}

#[test]
fn both_signals_zero_gives_zero_weights() {
    // R-164
    let w = run(0.0, 0.0, &risk(&[], true));

    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], Decimal::ZERO);
    assert_eq!(w[&sym("B")], Decimal::ZERO);
}

#[test]
fn ineligible_symbol_gets_zero_and_the_others_are_sized_without_it() {
    // R-164: A alone has u = 10 and u'Cu = 1, so w_A = 10 * 0.05 = 0.5.
    let w = run(1.0, 1.0, &risk(&["B"], true));

    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], dec("0.500000"));
    assert_eq!(w[&sym("B")], Decimal::ZERO);
}

#[test]
fn symbol_absent_from_vols_gets_zero_and_the_others_are_sized_without_it() {
    // R-164
    let w = run(1.0, 1.0, &risk(&[], false));

    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], dec("0.500000"));
    assert_eq!(w[&sym("B")], Decimal::ZERO);
}
