use chrono::NaiveDate;
use pt_core::types::Symbol;
use pt_core::view::{HistoryView, MemoryView};

fn d(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, month, day).unwrap()
}

fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

fn view() -> MemoryView {
    let a = sym("A");
    let mut v = MemoryView::new();
    v.push_adj_close(&a, d(1, 2), 10.0);
    v.push_adj_close(&a, d(1, 3), 11.0);
    v.push_adj_close(&a, d(1, 4), 12.0);
    v
}

// R-018: two entries, oldest first, never after the requested date.
#[test]
fn adj_closes_on_or_before_date_oldest_first() {
    let got = view().adj_closes(&sym("A"), d(1, 3), 10);
    assert_eq!(got, vec![(d(1, 2), 10.0), (d(1, 3), 11.0)]);
}

// R-018: the cap keeps the most recent entries.
#[test]
fn adj_closes_cap_keeps_latest() {
    let got = view().adj_closes(&sym("A"), d(1, 3), 1);
    assert_eq!(got, vec![(d(1, 3), 11.0)]);
}

// R-018: a date between pushed dates uses only earlier data.
#[test]
fn adj_closes_asof_between_dates() {
    let got = view().adj_closes(&sym("A"), d(1, 3).succ_opt().unwrap(), 10);
    assert_eq!(got.len(), 3);
    let got = view().adj_closes(&sym("A"), d(1, 1), 10);
    assert!(got.is_empty());
}

// R-018: oldest first even when pushed out of order.
#[test]
fn adj_closes_sorted_when_pushed_out_of_order() {
    let a = sym("A");
    let mut v = MemoryView::new();
    v.push_adj_close(&a, d(1, 4), 12.0);
    v.push_adj_close(&a, d(1, 2), 10.0);
    v.push_adj_close(&a, d(1, 3), 11.0);
    let got = v.adj_closes(&a, d(1, 4), 2);
    assert_eq!(got, vec![(d(1, 3), 11.0), (d(1, 4), 12.0)]);
}

// R-018: max of zero and unknown symbols give nothing.
#[test]
fn adj_closes_zero_max_and_unknown_symbol_are_empty() {
    assert!(view().adj_closes(&sym("A"), d(1, 4), 0).is_empty());
    assert!(view().adj_closes(&sym("B"), d(1, 4), 10).is_empty());
}

// R-018: symbols do not mix.
#[test]
fn adj_closes_are_per_symbol() {
    let mut v = view();
    v.push_adj_close(&sym("B"), d(1, 3), 99.0);
    assert_eq!(v.adj_closes(&sym("B"), d(1, 4), 10), vec![(d(1, 3), 99.0)]);
    assert_eq!(v.adj_closes(&sym("A"), d(1, 4), 10).len(), 3);
}
