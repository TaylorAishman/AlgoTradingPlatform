//! R-145: covariance is reported for every ordered pair of eligible symbols.

use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate, Weekday};
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::estimate::{estimate_risk, VolParams};
use pt_strategy::vol::ewma_cov;

const CLOSES_A: [f64; 12] = [
    100.0, 101.0, 99.5, 102.0, 103.5, 101.0, 104.0, 105.5, 103.0, 106.0, 107.5, 105.0,
];
const CLOSES_B: [f64; 12] = [
    50.0, 49.0, 50.5, 51.0, 49.5, 52.0, 51.0, 53.5, 52.0, 51.0, 53.0, 54.5,
];

fn params() -> VolParams {
    VolParams {
        com_days: 60.0,
        annualization: 261.0,
        min_obs: 5,
        max_obs: 8,
    }
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).expect("valid symbol")
}

fn sessions(n: usize) -> Vec<NaiveDate> {
    let mut d = NaiveDate::from_ymd_opt(2024, 1, 2).expect("date");
    let mut out = Vec::new();
    while out.len() < n {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(d);
        }
        d = d.succ_opt().expect("next day");
    }
    out
}

fn load(view: &mut MemoryView, s: &Symbol, closes: &[f64], skip: &[usize]) {
    for (i, (d, c)) in sessions(closes.len()).iter().zip(closes).enumerate() {
        if !skip.contains(&i) {
            view.push_adj_close(s, *d, *c);
        }
    }
}

fn returns(closes: &[f64]) -> Vec<f64> {
    closes.windows(2).map(|w| w[1] / w[0] - 1.0).collect()
}

#[test]
fn every_ordered_pair_of_eligible_symbols_has_a_symmetric_covariance() {
    // R-145
    let (a, b) = (sym("A"), sym("B"));
    let mut view = MemoryView::new();
    load(&mut view, &a, &CLOSES_A, &[]);
    load(&mut view, &b, &CLOSES_B, &[]);
    let asof = *sessions(12).last().expect("non-empty");

    let risk = estimate_risk(&[a.clone(), b.clone()], asof, &view, &params());

    let get = |x: &Symbol, y: &Symbol| {
        *risk
            .cov
            .get(&(x.clone(), y.clone()))
            .unwrap_or_else(|| panic!("missing cov key ({x:?}, {y:?})"))
    };
    assert_eq!(risk.cov.len(), 4);
    assert_eq!(get(&a, &b), get(&b, &a));
    for s in [&a, &b] {
        let v = risk.vols[s];
        assert!((get(s, s) - v * v).abs() < 1e-12, "variance of {s:?}");
    }
    let ra = returns(&CLOSES_A[3..]);
    let rb = returns(&CLOSES_B[3..]);
    let expected = ewma_cov(&ra, &rb, 60.0, 261.0).expect("eight returns");
    assert!((get(&a, &b) - expected).abs() < 1e-12);
}

#[test]
fn cov_has_no_key_involving_an_ineligible_symbol() {
    // R-145: G has a gap, Z has zero volatility, S has too little history
    let (a, b, g, z, s) = (sym("A"), sym("B"), sym("G"), sym("Z"), sym("S"));
    let mut view = MemoryView::new();
    load(&mut view, &a, &CLOSES_A, &[]);
    load(&mut view, &b, &CLOSES_B, &[]);
    load(&mut view, &g, &CLOSES_B, &[8]);
    load(&mut view, &z, &[50.0; 12], &[]);
    load(&mut view, &s, &CLOSES_A, &[]);
    let asof = *sessions(12).last().expect("non-empty");

    let symbols = [a.clone(), b.clone(), g.clone(), z.clone()];
    let risk = estimate_risk(&symbols, asof, &view, &params());

    assert!(risk.ineligible.contains_key(&g));
    assert!(risk.ineligible.contains_key(&z));
    let expected: BTreeSet<(Symbol, Symbol)> = [&a, &b]
        .iter()
        .flat_map(|x| [&a, &b].into_iter().map(move |y| ((*x).clone(), y.clone())))
        .collect();
    let got: BTreeSet<(Symbol, Symbol)> = risk.cov.keys().cloned().collect();
    assert_eq!(got, expected);
}

#[test]
fn a_single_eligible_symbol_has_only_its_own_variance() {
    // R-145
    let a = sym("A");
    let mut view = MemoryView::new();
    load(&mut view, &a, &CLOSES_A, &[]);
    let asof = *sessions(12).last().expect("non-empty");

    let risk = estimate_risk(std::slice::from_ref(&a), asof, &view, &params());

    assert_eq!(risk.cov.len(), 1);
    let v = risk.vols[&a];
    let c = risk.cov[&(a.clone(), a.clone())];
    assert!((c - v * v).abs() < 1e-12);
}
