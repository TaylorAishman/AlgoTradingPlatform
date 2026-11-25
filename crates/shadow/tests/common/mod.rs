//! The shadow fixture: a store and a config for the paper-vs-shadow comparison.
#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_config::config::{parse, Config};
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar, Side, Symbol};
use pt_store::db::Store;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, FillRow, LedgerEntry, OrderRow};
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

pub fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("valid decimal")
}

pub fn sym(s: &str) -> Symbol {
    Symbol::new(s).expect("valid symbol")
}

pub fn at(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, day, h, min, 0).unwrap()
}

/// Worked-example bars as (symbol, day of January 2024, open, close).
const BARS: [(&str, u32, i64, i64); 6] = [
    ("A", 2, 100, 100),
    ("A", 3, 101, 102),
    ("A", 4, 103, 104),
    ("B", 2, 50, 50),
    ("B", 3, 49, 48),
    ("B", 4, 47, 46),
];

pub fn config() -> Config {
    parse(
        "[[universe]]\nsymbol = \"A\"\nasset_class = \"equity\"\nshortable = true\n\n\
         [[universe]]\nsymbol = \"B\"\nasset_class = \"equity\"\nshortable = true\n\n\
         [cost.shadow]\nspread_bps = 10\n",
    )
    .expect("valid config")
}

pub fn order(
    id: &str,
    symbol: &str,
    side: Side,
    qty: u64,
    state: OrderState,
    broker_id: Option<&str>,
    reference: &str,
) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: sym(symbol),
        side,
        qty,
        leg: 1,
        state,
        broker_order_id: broker_id.map(str::to_string),
        filled_qty: if state == OrderState::Filled { qty } else { 0 },
        reference_price: dec(reference),
        trade_date: d(2024, 1, 3),
    }
}

pub fn fill(
    activity: &str,
    broker_id: &str,
    symbol: &str,
    side: Side,
    qty: u64,
    price: &str,
) -> FillRow {
    FillRow {
        activity_id: activity.to_string(),
        broker_order_id: broker_id.to_string(),
        symbol: sym(symbol),
        side,
        qty,
        price: dec(price),
        at: at(2024, 1, 3, 14, 31),
    }
}

pub fn shadow_store() -> Store {
    let store = Store::open_in_memory().expect("store");
    store
        .put_sessions(&[d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)])
        .unwrap();
    let mut rows = Vec::new();
    for (s, day, open, close) in BARS {
        for adjustment in [Adjustment::Raw, Adjustment::All] {
            rows.push(BarRow {
                bar: Bar {
                    symbol: sym(s),
                    date: d(2024, 1, day),
                    open: Price::new(Decimal::from(open)).unwrap(),
                    high: Price::new(Decimal::from(open.max(close))).unwrap(),
                    low: Price::new(Decimal::from(open.min(close))).unwrap(),
                    close: Price::new(Decimal::from(close)).unwrap(),
                    volume: 1_000_000,
                },
                adjustment,
                feed: "sip".to_string(),
            });
        }
    }
    store.upsert_bars(&rows, at(2024, 1, 5, 0, 0)).unwrap();

    let entry =
        |date, kind: &str, symbol: Option<&str>, qty: i64, price: Option<&str>, cash: &str| {
            LedgerEntry {
                book: Book::Paper,
                date,
                kind: kind.to_string(),
                symbol: symbol.map(sym),
                qty,
                price: price.map(dec),
                cash_delta: dec(cash),
                ref_id: None,
            }
        };
    store
        .append_ledger(&entry(d(2024, 1, 2), "opening", None, 0, None, "100000"))
        .unwrap();
    store
        .append_ledger(&entry(
            d(2024, 1, 3),
            "fill",
            Some("A"),
            500,
            Some("101.05"),
            "-50525",
        ))
        .unwrap();
    store
        .append_ledger(&entry(
            d(2024, 1, 3),
            "fill",
            Some("B"),
            -500,
            Some("48.97"),
            "24485",
        ))
        .unwrap();

    store
        .insert_order(&order(
            "o-A",
            "A",
            Side::Buy,
            500,
            OrderState::Filled,
            Some("b-A"),
            "100",
        ))
        .unwrap();
    store
        .insert_order(&order(
            "o-B",
            "B",
            Side::Sell,
            500,
            OrderState::Filled,
            Some("b-B"),
            "50",
        ))
        .unwrap();
    store
        .insert_fill(&fill("f-A", "b-A", "A", Side::Buy, 500, "101.05"))
        .unwrap();
    store
        .insert_fill(&fill("f-B", "b-B", "B", Side::Sell, 500, "48.97"))
        .unwrap();

    for (date, equity, cash, positions, recorded) in [
        (
            d(2024, 1, 2),
            "100000",
            "100000",
            "0",
            at(2024, 1, 2, 21, 0),
        ),
        (
            d(2024, 1, 3),
            "100960",
            "73960",
            "27000",
            at(2024, 1, 3, 21, 0),
        ),
        (
            d(2024, 1, 4),
            "102960",
            "73960",
            "29000",
            at(2024, 1, 4, 21, 0),
        ),
    ] {
        store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date,
                cash: dec(cash),
                positions_value: dec(positions),
                equity: dec(equity),
                recorded_at: recorded,
            })
            .unwrap();
    }
    store
}
