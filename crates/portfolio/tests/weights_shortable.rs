//! R-163: a short signal in a non-shortable instrument becomes flat before scaling.

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

fn run(sig_a: f64, sig_b: f64, shortable: &[&str]) -> Weights {
    let mut signals = BTreeMap::new();
    signals.insert(sym("A"), sig_a);
    signals.insert(sym("B"), sig_b);
    let shortable: BTreeSet<Symbol> = shortable.iter().map(|s| sym(s)).collect();
    let p = SizingParams {
        target_vol: 0.1,
        max_weight: 1.0,
        max_gross: 1.0,
    };
    target_weights(&signals, &risk(), &shortable, &p)
}

#[test]
fn short_signal_in_non_shortable_symbol_is_flat() {
    // R-163: only A shortable; B's short is flattened, so A alone meets the vol target.
    let w = run(1.0, -1.0, &["A"]);

    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], dec("1.000000"));
    assert_eq!(w[&sym("B")], dec("0.000000"));
}

#[test]
fn flattening_happens_before_scaling_not_after() {
    // R-163: with A short and non-shortable, B alone is scaled to the vol target (0.5).
    let w = run(-1.0, 1.0, &["B"]);

    assert_eq!(w[&sym("A")], dec("0.000000"));
    assert_eq!(w[&sym("B")], dec("0.500000"));
}

#[test]
fn long_signal_in_non_shortable_symbol_is_kept() {
    // R-163: only shorts are flattened; longs are sized as usual.
    let w = run(1.0, 1.0, &["A"]);

    assert_eq!(w[&sym("A")], dec("0.666667"));
    assert_eq!(w[&sym("B")], dec("0.333333"));
}

#[test]
fn shorts_in_shortable_symbols_are_kept() {
    // R-163: with both shortable nothing is flattened (same as R-162 case).
    let w = run(1.0, -1.0, &["A", "B"]);

    assert_eq!(w[&sym("A")], dec("0.666667"));
    assert_eq!(w[&sym("B")], dec("-0.333333"));
}

#[test]
fn empty_shortable_set_flattens_every_short() {
    // R-163: nothing is shortable, so both shorts become flat.
    let w = run(-1.0, -1.0, &[]);

    assert_eq!(w[&sym("A")], dec("0.000000"));
    assert_eq!(w[&sym("B")], dec("0.000000"));
}
