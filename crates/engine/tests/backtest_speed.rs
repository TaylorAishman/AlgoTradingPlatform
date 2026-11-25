//! K23 (R-210): a ten-year backtest runs within the time limit.

mod common;

use chrono::{Datelike, NaiveDate, Weekday};
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_calendar::schedule::Rebalance;
use pt_core::cost::CostSpec;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::{HistoryView, MemoryView};
use pt_engine::decision::DecisionParams;
use pt_engine::schedule::run_backtest;
use pt_engine::simulate::SimConfig;
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::VolParams;
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::time::{Duration, Instant};

const SYMBOL_COUNT: usize = 40;
const SESSIONS: usize = 2800;
const LIMIT: Duration = Duration::from_secs(30);

struct AllLong;

impl Strategy for AllLong {
    fn id(&self) -> &str {
        "all-long"
    }
    fn params_canonical(&self) -> String {
        String::new()
    }
    fn signals(
        &self,
        universe: &[Symbol],
        asof: NaiveDate,
        _view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        Ok(SignalSet {
            asof,
            signals: universe.iter().map(|s| (s.clone(), 1.0)).collect(),
            ineligible: BTreeMap::new(),
        })
    }
}

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn params() -> DecisionParams {
    DecisionParams {
        vol: VolParams {
            com_days: 20.0,
            annualization: 261.0,
            min_obs: 10,
            max_obs: 60,
        },
        sizing: SizingParams {
            target_vol: 0.10,
            max_weight: 0.2,
            max_gross: 1.0,
        },
        limits: RiskLimits {
            max_order_notional: dec(1_000_000),
            max_weight: dd("0.2"),
            max_gross: dec(1),
            daily_loss_limit: dd("0.03"),
            max_drawdown: dd("0.15"),
        },
    }
}

fn sessions() -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut day = d(2022, 1, 3);
    while out.len() < SESSIONS {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(day);
        }
        day = day.succ_opt().unwrap();
    }
    out
}

/// Seeded synthetic prices: a linear congruential walk per symbol.
fn view(dates: &[NaiveDate], names: &[String]) -> MemoryView {
    let mut v = MemoryView::new();
    let mut state: u64 = 12345;
    for (k, s) in names.iter().enumerate() {
        let mut px: i64 = 100 + (k as i64 % 10) * 10;
        for date in dates {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let step = ((state >> 33) % 5) as i64 - 2;
            let open = px;
            px = (px + step).max(20);
            v.push_exec(bar(s, *date, open, px));
            v.push_adj_close(&sym(s), *date, px as f64);
        }
    }
    v
}

fn rebalances(dates: &[NaiveDate]) -> Vec<Rebalance> {
    (30..dates.len() - 1)
        .step_by(21)
        .map(|i| Rebalance {
            signal_date: dates[i],
            trade_date: dates[i + 1],
        })
        .collect()
}

#[test]
fn ten_year_backtest_of_forty_symbols_finishes_within_thirty_seconds() {
    // R-210
    let names: Vec<String> = (0..SYMBOL_COUNT).map(|i| format!("S{i:02}")).collect();
    let dates = sessions();
    let v = view(&dates, &names);
    let cal = TradingCalendar::new(dates.clone()).unwrap();
    let universe: Vec<Symbol> = names.iter().map(|s| sym(s)).collect();
    let shortable: BTreeSet<Symbol> = universe.iter().cloned().collect();
    let cfg = SimConfig {
        start: dates[0],
        end: dates[SESSIONS - 1],
        initial_capital: dec(1_000_000),
        cost: CostSpec::zero(),
        limits: params().limits,
        universe,
    };
    let schedule = rebalances(&dates);
    assert!(schedule.len() >= 120, "a monthly schedule over ten years");
    let p = params();

    let started = Instant::now();
    let result = run_backtest(&cfg, &schedule, &shortable, &AllLong, &v, &cal, &p).unwrap();
    let elapsed = started.elapsed();

    assert_eq!(result.equity.len(), SESSIONS);
    assert!(!result.fills.is_empty(), "the schedule must trade");
    assert!(
        elapsed < LIMIT,
        "backtest took {elapsed:?}, limit {LIMIT:?}"
    );
}
