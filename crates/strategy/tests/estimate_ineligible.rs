//! R-144: zero-volatility and gapped symbols are ineligible.

use chrono::{Datelike, NaiveDate, Weekday};
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::estimate::{estimate_risk, VolParams};
use pt_strategy::vol::ewma_vol;

const CLOSES: [f64; 12] = [
    100.0, 101.0, 99.5, 102.0, 103.5, 101.0, 104.0, 105.5, 103.0, 106.0, 107.5, 105.0,
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

/// Push `closes` on the first sessions, skipping the session indexes in `skip`.
fn load(view: &mut MemoryView, s: &Symbol, closes: &[f64], skip: &[usize]) {
    for (i, (d, c)) in sessions(closes.len()).iter().zip(closes).enumerate() {
        if !skip.contains(&i) {
            view.push_adj_close(s, *d, *c);
        }
    }
}

#[test]
fn constant_closes_are_ineligible_as_zero_vol() {
    // R-144
    let (a, z) = (sym("A"), sym("Z"));
    let mut view = MemoryView::new();
    load(&mut view, &a, &CLOSES, &[]);
    load(&mut view, &z, &[50.0; 12], &[]);
    let asof = *sessions(12).last().expect("non-empty");

    let risk = estimate_risk(&[a.clone(), z.clone()], asof, &view, &params());

    assert_eq!(
        risk.ineligible.get(&z).map(String::as_str),
        Some("zero_vol")
    );
    assert!(!risk.vols.contains_key(&z));
    assert!(risk.vols.contains_key(&a));
    assert!(!risk.ineligible.contains_key(&a));
}

#[test]
fn a_missing_session_in_the_window_makes_that_symbol_a_gap_and_spares_the_other() {
    // R-144: B lacks session 8, which A has and which is inside A's window
    let (a, b) = (sym("A"), sym("B"));
    let mut view = MemoryView::new();
    load(&mut view, &a, &CLOSES, &[]);
    load(&mut view, &b, &CLOSES, &[8]);
    let asof = *sessions(12).last().expect("non-empty");

    let risk = estimate_risk(&[a.clone(), b.clone()], asof, &view, &params());

    assert_eq!(risk.ineligible.get(&b).map(String::as_str), Some("gap"));
    assert!(!risk.vols.contains_key(&b));
    assert!(!risk.ineligible.contains_key(&a));
    let rets: Vec<f64> = CLOSES[3..].windows(2).map(|w| w[1] / w[0] - 1.0).collect();
    let expected = ewma_vol(&rets, 60.0, 261.0).expect("eight returns");
    let got = *risk.vols.get(&a).expect("A is still estimated");
    assert!((got - expected).abs() < 1e-12, "got {got}, want {expected}");
}
