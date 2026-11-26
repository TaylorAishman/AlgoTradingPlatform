//! R-448: a symbol whose volatility is not finite is ineligible.

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

fn load(view: &mut MemoryView, s: &Symbol, closes: &[f64]) {
    for (d, c) in sessions(closes.len()).iter().zip(closes) {
        view.push_adj_close(s, *d, *c);
    }
}

#[test]
fn a_zero_close_makes_the_symbol_non_finite_and_leaves_the_other_untouched() {
    // R-448
    let (a, b) = (sym("A"), sym("B"));
    let mut zeroed = CLOSES;
    zeroed[6] = 0.0;
    let mut view = MemoryView::new();
    load(&mut view, &a, &zeroed);
    load(&mut view, &b, &CLOSES);
    let asof = *sessions(12).last().expect("non-empty");

    let risk = estimate_risk(&[a.clone(), b.clone()], asof, &view, &params());

    assert_eq!(
        risk.ineligible.get(&a).map(String::as_str),
        Some("non_finite")
    );
    assert!(!risk.vols.contains_key(&a));
    assert!(risk.cov.keys().all(|(x, y)| *x != a && *y != a));
    assert!(!risk.ineligible.contains_key(&b));
    let rets: Vec<f64> = CLOSES[3..].windows(2).map(|w| w[1] / w[0] - 1.0).collect();
    let expected = ewma_vol(&rets, 60.0, 261.0).expect("eight returns");
    let got = *risk.vols.get(&b).expect("B is still estimated");
    assert!((got - expected).abs() < 1e-12, "got {got}, want {expected}");
    assert!(risk.cov.contains_key(&(b.clone(), b)));
}
