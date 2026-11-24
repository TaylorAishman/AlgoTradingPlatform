//! Shared fixtures: the worked example's symbols, dates and execution bars.
#![allow(dead_code)]

use chrono::NaiveDate;
use pt_core::money::Price;
use pt_core::types::{Bar, OrderIntent, Side, Symbol};
use pt_core::view::MemoryView;
use rust_decimal::Decimal;

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

pub fn dec(n: i64) -> Decimal {
    Decimal::from(n)
}

pub fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

pub fn price(n: i64) -> Price {
    Price::new(dec(n)).unwrap()
}

pub fn bar(symbol: &str, date: NaiveDate, open: i64, close: i64) -> Bar {
    Bar {
        symbol: sym(symbol),
        date,
        open: price(open),
        high: price(open.max(close)),
        low: price(open.min(close)),
        close: price(close),
        volume: 1_000_000,
    }
}

/// Execution bars of the worked example, as (symbol, day of January 2024, open, close).
pub const WORKED_BARS: [(&str, u32, i64, i64); 6] = [
    ("A", 2, 100, 100),
    ("A", 3, 101, 102),
    ("A", 4, 103, 104),
    ("B", 2, 50, 50),
    ("B", 3, 49, 48),
    ("B", 4, 47, 46),
];

/// The worked example's view; bars of `skip` (symbol, day) are left out.
pub fn worked_view_without(skip: &[(&str, u32)]) -> MemoryView {
    let mut view = MemoryView::new();
    for (s, day, open, close) in WORKED_BARS {
        if skip.contains(&(s, day)) {
            continue;
        }
        view.push_exec(bar(s, d(2024, 1, day), open, close));
    }
    view
}

pub fn worked_view() -> MemoryView {
    worked_view_without(&[])
}

pub fn order(symbol: &str, side: Side, qty: u64, leg: u8, reference: i64) -> OrderIntent {
    OrderIntent::new(sym(symbol), side, qty, leg, price(reference)).unwrap()
}
