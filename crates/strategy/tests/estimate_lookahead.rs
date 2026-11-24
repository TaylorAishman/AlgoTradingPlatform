//! R-146: risk estimates do not change when data after `asof` changes.

use chrono::{Datelike, NaiveDate, Weekday};
use pt_core::types::Symbol;
use pt_core::view::MemoryView;
use pt_strategy::estimate::{estimate_risk, VolParams};

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

/// Closes through session index 11 (the as-of date) are shared; `later`
/// supplies the closes for the following sessions.
fn build(later_a: &[f64], later_b: &[f64], b_skips_later: bool) -> MemoryView {
    let (a, b) = (sym("A"), sym("B"));
    let mut view = MemoryView::new();
    let days = sessions(12 + later_a.len());
    for (i, d) in days.iter().enumerate() {
        if i < 12 {
            view.push_adj_close(&a, *d, CLOSES[i]);
            view.push_adj_close(&b, *d, CLOSES[i] * 0.5 + (i % 3) as f64);
        } else {
            view.push_adj_close(&a, *d, later_a[i - 12]);
            if !b_skips_later {
                view.push_adj_close(&b, *d, later_b[i - 12]);
            }
        }
    }
    view
}

#[test]
fn estimates_are_identical_when_closes_after_asof_differ() {
    // R-146
    let asof = sessions(12)[11];
    let syms = [sym("A"), sym("B")];
    let base = build(&[106.0, 107.0, 108.0], &[20.0, 21.0, 22.0], false);
    let other = build(&[1.0, 500.0, 3.0], &[900.0, 1.0, 40.0], false);

    let r1 = estimate_risk(&syms, asof, &base, &params());
    let r2 = estimate_risk(&syms, asof, &other, &params());

    assert!(!r1.vols.is_empty(), "the estimate must not be vacuous");
    assert_eq!(r1, r2);
}

#[test]
fn estimates_are_identical_with_or_without_later_data() {
    // R-146: adding data after asof, even with a symbol missing later sessions
    let asof = sessions(12)[11];
    let syms = [sym("A"), sym("B")];
    let none = build(&[], &[], false);
    let later = build(&[1.0, 500.0, 3.0], &[], true);

    let r1 = estimate_risk(&syms, asof, &none, &params());
    let r2 = estimate_risk(&syms, asof, &later, &params());

    assert!(r1.ineligible.is_empty());
    assert_eq!(r1, r2);
}
