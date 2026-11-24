use chrono::NaiveDate;
use pt_core::money::Price;
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{Bar, Symbol};
use pt_core::view::{HistoryView, MarketView};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 1, day).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

/// A fixed-answer view, to prove the traits can be implemented outside pt-core.
struct Fake;

impl HistoryView for Fake {
    fn adj_closes(&self, _symbol: &Symbol, asof: NaiveDate, max: usize) -> Vec<(NaiveDate, f64)> {
        let all = [(d(2), 1.0), (d(3), 2.0), (d(4), 3.0)];
        let ok: Vec<_> = all.into_iter().filter(|(x, _)| *x <= asof).collect();
        let skip = ok.len().saturating_sub(max);
        ok[skip..].to_vec()
    }
}

impl MarketView for Fake {
    fn exec_bar(&self, symbol: &Symbol, date: NaiveDate) -> Option<Bar> {
        let p = Price::new(Decimal::new(100, 0)).unwrap();
        (date == d(2)).then(|| Bar {
            symbol: symbol.clone(),
            date,
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 500,
        })
    }
    fn adv_shares(&self, _symbol: &Symbol, _asof: NaiveDate, sessions: usize) -> Option<f64> {
        (sessions <= 2).then_some(500.0)
    }
}

fn history_len(v: &dyn HistoryView, asof: NaiveDate, max: usize) -> usize {
    v.adj_closes(&sym("A"), asof, max).len()
}

// R-018: the view interface is usable as trait objects
#[test]
fn history_view_works_as_trait_object() {
    assert_eq!(history_len(&Fake, d(3), 10), 2);
    assert_eq!(history_len(&Fake, d(3), 1), 1);
}

// R-018: a MarketView is usable where a HistoryView is expected (supertrait)
#[test]
fn market_view_is_a_history_view() {
    fn takes_market(v: &dyn MarketView) -> usize {
        history_len_generic(v)
    }
    fn history_len_generic<V: HistoryView + ?Sized>(v: &V) -> usize {
        v.adj_closes(&sym("A"), d(4), 10).len()
    }
    assert_eq!(takes_market(&Fake), 3);
}

// R-018: exec_bar returns the bar for the date or None
#[test]
fn market_view_exec_bar_and_adv_through_trait_object() {
    let v: &dyn MarketView = &Fake;
    assert_eq!(v.exec_bar(&sym("A"), d(2)).unwrap().date, d(2));
    assert!(v.exec_bar(&sym("A"), d(3)).is_none());
    assert_eq!(v.adv_shares(&sym("A"), d(4), 2), Some(500.0));
    assert_eq!(v.adv_shares(&sym("A"), d(4), 3), None);
}

// R-018: signal set carries asof, signals and ineligible reasons
#[test]
fn signal_set_holds_signals_and_ineligible() {
    let mut signals = BTreeMap::new();
    signals.insert(sym("A"), 0.5_f64);
    let mut ineligible = BTreeMap::new();
    ineligible.insert(sym("B"), "short history".to_string());
    let s = SignalSet {
        asof: d(3),
        signals,
        ineligible,
    };
    assert_eq!(s.asof, d(3));
    assert_eq!(s.signals[&sym("A")], 0.5);
    assert_eq!(s.ineligible[&sym("B")], "short history");
}

// R-018: risk estimates carry vols and pairwise covariances keyed by symbol pair
#[test]
fn risk_estimates_hold_vols_and_cov() {
    let mut vols = BTreeMap::new();
    vols.insert(sym("A"), 0.2_f64);
    let mut cov = BTreeMap::new();
    cov.insert((sym("A"), sym("B")), 0.01_f64);
    let r = RiskEstimates {
        asof: d(4),
        vols,
        cov,
        ineligible: BTreeMap::new(),
    };
    assert_eq!(r.asof, d(4));
    assert_eq!(r.vols[&sym("A")], 0.2);
    assert_eq!(r.cov[&(sym("A"), sym("B"))], 0.01);
    assert!(r.ineligible.is_empty());
}
