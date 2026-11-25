//! The replay fixture (requirements section P): the live fixture's bars for SPY, TLT and BIL,
//! the live fixture's config, a `Tsmom` strategy and one paper decision row.
#![allow(dead_code)]

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc, Weekday};
use pt_calendar::calendar::TradingCalendar;
use pt_config::config::{parse, Config};
use pt_config::hash::config_hash;
use pt_core::money::{Money, Price};
use pt_core::types::{Adjustment, Bar, Positions, PriceBasis, Symbol};
use pt_data::view::StoreView;
use pt_engine::decision::{
    decide, decision_to_json, params_from_config, record_to_json, DecisionInputs, DecisionRecord,
};
use pt_store::db::Store;
use pt_store::market::BarRow;
use pt_store::trading::{DecisionRow, Mode};
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use rust_decimal::Decimal;
use std::collections::BTreeSet;
use std::sync::Arc;

pub struct Fixture {
    pub store: Store,
    pub cfg: Config,
    pub strategy: Tsmom,
    pub decision_id: i64,
    pub row: DecisionRow,
    pub record: DecisionRecord,
}

pub fn sym(s: &str) -> Symbol {
    Symbol::new(s).unwrap()
}

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// The fetch time of the original bars and of the decision's data.
pub fn t1() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 7, 0, 0).unwrap()
}

/// A later fetch time, for restatements.
pub fn t2() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 15, 7, 0, 0).unwrap()
}

/// The 2024 sessions through March: weekdays except New Year's Day, MLK Day and Presidents' Day.
pub fn sessions() -> Vec<NaiveDate> {
    let holidays = [d(2024, 1, 1), d(2024, 1, 15), d(2024, 2, 19)];
    let mut out = Vec::new();
    let mut day = d(2024, 1, 1);
    while day <= d(2024, 3, 29) {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) && !holidays.contains(&day) {
            out.push(day);
        }
        day = day.succ_opt().unwrap();
    }
    out
}

pub fn config() -> Config {
    parse(
        r#"
[data]
feed = "sip"
start = "2024-01-02"

[strategy]
id = "tsmom-v1"
lookback_months = [1]
risk_free_symbol = "BIL"
vol_com_days = 60
vol_annualization = 261
vol_min_obs = 5
vol_max_obs = 20

[sizing]
target_vol = "0.10"
max_weight = "0.5"
max_gross = "1.0"

[risk]
max_order_notional = "60000"

[[universe]]
symbol = "SPY"
asset_class = "equity"
shortable = true

[[universe]]
symbol = "TLT"
asset_class = "treasury"
shortable = true
"#,
    )
    .expect("the fixture config parses")
}

fn price(x: Decimal) -> Price {
    Price::new(x).unwrap()
}

/// The close of `symbol` on the i-th of the 41 sessions 2024-01-02..2024-02-29.
fn close(symbol: &str, i: i64) -> Decimal {
    let i_d = Decimal::from(i);
    match symbol {
        "SPY" => Decimal::from(100 + i + (i % 2)),
        "TLT" => {
            Decimal::from(100) - Decimal::new(5, 1) * i_d
                + Decimal::new(2, 1) * Decimal::from(i % 3)
        }
        _ => Decimal::from(100) + Decimal::new(1, 2) * i_d,
    }
}

/// One series as bars of the given adjustment, with `scale` applied to every close from `from_date` on.
pub fn bar_rows(
    symbol: &str,
    adjustment: Adjustment,
    from_date: NaiveDate,
    scale: Decimal,
) -> Vec<BarRow> {
    let days: Vec<NaiveDate> = sessions()
        .into_iter()
        .filter(|s| *s >= d(2024, 1, 2) && *s <= d(2024, 2, 29))
        .collect();
    days.iter()
        .enumerate()
        .map(|(i, date)| {
            let i = i as i64;
            let s = if *date >= from_date {
                scale
            } else {
                Decimal::ONE
            };
            let c = close(symbol, i) * s;
            let o = if i == 0 {
                c
            } else {
                close(symbol, i - 1)
                    * if days[i as usize - 1] >= from_date {
                        scale
                    } else {
                        Decimal::ONE
                    }
            };
            BarRow {
                bar: Bar {
                    symbol: sym(symbol),
                    date: *date,
                    open: price(o),
                    high: price(o.max(c)),
                    low: price(o.min(c)),
                    close: price(c),
                    volume: 1_000_000,
                },
                adjustment,
                feed: "sip".to_string(),
            }
        })
        .collect()
}

pub fn build() -> Fixture {
    let store = Store::open_in_memory().unwrap();
    store.put_sessions(&sessions()).unwrap();
    for symbol in ["SPY", "TLT", "BIL"] {
        for adj in [Adjustment::Raw, Adjustment::All] {
            store
                .upsert_bars(&bar_rows(symbol, adj, d(2100, 1, 1), Decimal::ONE), t1())
                .unwrap();
        }
    }
    let cfg = config();
    let strategy = Tsmom::new(
        TsmomParams {
            lookback_months: cfg.strategy.lookback_months.clone(),
            risk_free_symbol: sym(&cfg.strategy.risk_free_symbol),
        },
        Arc::new(TradingCalendar::new(sessions()).unwrap()),
    );
    let universe = vec![sym("SPY"), sym("TLT")];
    let inputs = DecisionInputs {
        signal_date: d(2024, 2, 29),
        trade_date: d(2024, 3, 1),
        ref_date: d(2024, 2, 29),
        equity: Money(Decimal::from(100_000)),
        positions: Positions::new(),
        universe: universe.clone(),
        shortable: universe.iter().cloned().collect::<BTreeSet<_>>(),
    };
    let record = DecisionRecord {
        inputs: inputs.clone(),
        data_asof: t1(),
        feed: "sip".to_string(),
        config_hash: config_hash(&cfg),
        code_version: "test".to_string(),
        bootstrap: false,
    };
    let symbols = vec![sym("SPY"), sym("TLT"), sym("BIL")];
    let view = StoreView::load(&store, &symbols, "sip", PriceBasis::Raw, Some(t1())).unwrap();
    let decision = decide(&inputs, &strategy, &view, &params_from_config(&cfg)).unwrap();
    let row = DecisionRow {
        id: 0,
        strategy_id: "tsmom-v1".to_string(),
        signal_date: d(2024, 2, 29),
        trade_date: Some(d(2024, 3, 1)),
        mode: Mode::Paper,
        status: "planned".to_string(),
        inputs_json: record_to_json(&record),
        outputs_json: decision_to_json(&decision),
        created_at: t1(),
    };
    let decision_id = store.insert_decision(&row).unwrap();
    Fixture {
        store,
        cfg,
        strategy,
        decision_id,
        row,
        record,
    }
}
