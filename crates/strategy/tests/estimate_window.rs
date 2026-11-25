//! R-143: risk estimates need a minimum history and use at most a maximum window.

use chrono::{Datelike, NaiveDate, Weekday};
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::estimate::{estimate_risk, VolParams};
use pt_strategy::vol::ewma_vol;

const CLOSES: [f64; 20] = [
    100.0, 101.0, 99.5, 102.0, 103.5, 101.0, 104.0, 105.5, 103.0, 106.0, 107.5, 105.0, 108.0,
    109.5, 107.0, 110.0, 111.5, 109.0, 112.0, 113.5,
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

fn returns(closes: &[f64]) -> Vec<f64> {
    closes.windows(2).map(|w| w[1] / w[0] - 1.0).collect()
}

/// Load the first `n` of `closes` for `s` on the first `n` sessions; return the last date.
fn load(view: &mut MemoryView, s: &Symbol, closes: &[f64], n: usize) -> NaiveDate {
    let days = sessions(n);
    for (d, c) in days.iter().zip(closes) {
        view.push_adj_close(s, *d, *c);
    }
    *days.last().expect("non-empty")
}

#[test]
fn enough_history_gives_a_volatility_and_short_history_is_ineligible() {
    // R-143
    let (a, b) = (sym("A"), sym("B"));
    let mut view = MemoryView::new();
    let asof = load(&mut view, &a, &CLOSES, 7);
    load(&mut view, &b, &CLOSES, 4);

    let risk = estimate_risk(&[a.clone(), b.clone()], asof, &view, &params());

    assert_eq!(risk.asof, asof);
    let expected = ewma_vol(&returns(&CLOSES[..7]), 60.0, 261.0).expect("six returns");
    let got = *risk.vols.get(&a).expect("A has a volatility");
    assert!((got - expected).abs() < 1e-12, "got {got}, want {expected}");
    assert!(!risk.vols.contains_key(&b));
    assert_eq!(
        risk.ineligible.get(&b).map(String::as_str),
        Some("insufficient_history")
    );
    assert!(!risk.ineligible.contains_key(&a));
}

#[test]
fn only_the_last_max_obs_returns_are_used() {
    // R-143
    let a = sym("A");
    let mut view = MemoryView::new();
    let asof = load(&mut view, &a, &CLOSES, 20);

    let risk = estimate_risk(std::slice::from_ref(&a), asof, &view, &params());

    let last_eight = returns(&CLOSES[CLOSES.len() - 9..]);
    assert_eq!(last_eight.len(), 8);
    let expected = ewma_vol(&last_eight, 60.0, 261.0).expect("eight returns");
    let all = ewma_vol(&returns(&CLOSES), 60.0, 261.0).expect("nineteen returns");
    assert!(
        (expected - all).abs() > 1e-6,
        "fixture must tell windows apart"
    );
    let got = *risk.vols.get(&a).expect("A has a volatility");
    assert!((got - expected).abs() < 1e-12, "got {got}, want {expected}");
}

#[test]
fn exactly_min_obs_returns_is_enough() {
    // R-143 boundary: 6 closes give 5 returns
    let a = sym("A");
    let mut view = MemoryView::new();
    let asof = load(&mut view, &a, &CLOSES, 6);

    let risk = estimate_risk(std::slice::from_ref(&a), asof, &view, &params());

    assert!(risk.vols.contains_key(&a));
    assert!(risk.ineligible.is_empty());
}

#[test]
fn one_return_fewer_than_min_obs_is_insufficient() {
    // R-143 boundary: 5 closes give 4 returns
    let a = sym("A");
    let mut view = MemoryView::new();
    let asof = load(&mut view, &a, &CLOSES, 5);

    let risk = estimate_risk(std::slice::from_ref(&a), asof, &view, &params());

    assert!(risk.vols.is_empty());
    assert_eq!(
        risk.ineligible.get(&a).map(String::as_str),
        Some("insufficient_history")
    );
}
