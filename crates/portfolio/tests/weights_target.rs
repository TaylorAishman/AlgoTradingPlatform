//! R-160: weights are inverse-volatility positions scaled to the portfolio volatility target.

use pt_core::signal::RiskEstimates;
use pt_core::types::Symbol;
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

#[test]
fn long_and_short_are_inverse_vol_scaled_to_target() {
    // R-160
    let mut signals = BTreeMap::new();
    signals.insert(sym("A"), 1.0);
    signals.insert(sym("B"), -1.0);
    let shortable: BTreeSet<Symbol> = [sym("A"), sym("B")].into_iter().collect();
    let p = SizingParams {
        target_vol: 0.1,
        max_weight: 1.0,
        max_gross: 2.0,
    };

    let w = target_weights(&signals, &risk(), &shortable, &p);

    assert_eq!(w.len(), 2);
    assert_eq!(w[&sym("A")], dec("0.707107"));
    assert_eq!(w[&sym("B")], dec("-0.353553"));
}
