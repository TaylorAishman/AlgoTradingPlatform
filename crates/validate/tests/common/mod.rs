//! The research fixture shared by the validation tests: a store holding synthetic
//! bars for S1 to S5 and BIL on every weekday of 2024, and a matching config.
#![allow(dead_code)]

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc, Weekday};
use pt_config::config::{parse, Config};
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_store::db::Store;
use pt_store::market::BarRow;
use pt_validate::runner::BacktestRequest;
use rust_decimal::Decimal;

pub const SYMBOLS: [&str; 5] = ["S1", "S2", "S3", "S4", "S5"];
pub const FEED: &str = "csv:synth";

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

pub fn fetched_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap()
}

/// Every weekday of 2024.
pub fn sessions() -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut d = date(2024, 1, 1);
    while d.year() == 2024 {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(d);
        }
        d = d.succ_opt().expect("next day");
    }
    out
}

/// The config of the research fixture; it passes `pt_config` validation.
pub fn config() -> Config {
    let mut text = String::new();
    for s in SYMBOLS {
        text.push_str(&format!(
            "[[universe]]\nsymbol = \"{s}\"\nasset_class = \"equity\"\nshortable = true\n\n"
        ));
    }
    text.push_str(
        "[data]\nfeed = \"csv:synth\"\n\n\
         [strategy]\nlookback_months = [1]\nvol_min_obs = 10\nvol_max_obs = 40\n\n\
         [sizing]\nmax_weight = 0.3\n\n\
         [risk]\nmax_order_notional = 1000000\n",
    );
    parse(&text).expect("fixture config parses")
}

/// `tsmom` from 2024-03-01 to 2024-12-30 with capital 100000.
pub fn request() -> BacktestRequest {
    BacktestRequest {
        strategy: "tsmom".into(),
        from: date(2024, 3, 1),
        to: date(2024, 12, 30),
        initial_capital: Decimal::new(100_000, 0),
    }
}

/// Close prices of one symbol, one per session, from a fixed-seed linear
/// congruential generator: daily returns between -2% and +2% (BIL: +0.01%),
/// rounded to two decimal places.
fn closes(seed: u64, n: usize, bil: bool) -> Vec<Decimal> {
    let mut state = seed;
    let mut price = Decimal::new(100, 0);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let bp: i64 = if bil {
            1
        } else {
            i64::try_from((state >> 33) % 4001).expect("small") - 2000
        };
        price = (price * (Decimal::ONE + Decimal::new(bp, 4))).round_dp(2);
        out.push(price);
    }
    out
}

fn price(d: Decimal) -> Price {
    Price::new(d).expect("positive price")
}

/// The fixture rows of one symbol for one adjustment, with the bars of the dates
/// in `skip` left out. `raw_scale` multiplies every price of the series.
fn rows(
    symbol: &str,
    seed: u64,
    adjustment: Adjustment,
    scale: Decimal,
    skip: &[NaiveDate],
) -> Vec<BarRow> {
    let days = sessions();
    let cl = closes(seed, days.len(), symbol == "BIL");
    let mut prev = Decimal::new(100, 0);
    let mut out = Vec::new();
    for (day, close) in days.iter().zip(cl) {
        let open = prev;
        prev = close;
        if skip.contains(day) {
            continue;
        }
        out.push(BarRow {
            bar: Bar {
                symbol: Symbol::new(symbol).expect("symbol"),
                date: *day,
                open: price(open * scale),
                high: price(open.max(close) * scale),
                low: price(open.min(close) * scale),
                close: price(close * scale),
                volume: 1_000_000,
            },
            adjustment,
            feed: FEED.into(),
        });
    }
    out
}

/// The research fixture store. Bars of `skip` (symbol, date) pairs are left out of
/// both series; the raw series is multiplied by `raw_scale` (1 for identical series).
pub fn store_with(skip: &[(&str, NaiveDate)], raw_scale: Decimal) -> Store {
    let store = Store::open_in_memory().expect("in-memory store");
    store.put_sessions(&sessions()).expect("sessions");
    let mut all = Vec::new();
    for (i, sym) in SYMBOLS.iter().copied().chain(["BIL"]).enumerate() {
        let skipped: Vec<NaiveDate> = skip
            .iter()
            .filter(|(s, _)| *s == sym)
            .map(|(_, d)| *d)
            .collect();
        let seed = 1_000 + u64::try_from(i).expect("small");
        all.extend(rows(sym, seed, Adjustment::All, Decimal::ONE, &skipped));
        all.extend(rows(sym, seed, Adjustment::Raw, raw_scale, &skipped));
    }
    store.upsert_bars(&all, fetched_at()).expect("bars");
    store
}

pub fn store() -> Store {
    store_with(&[], Decimal::ONE)
}
