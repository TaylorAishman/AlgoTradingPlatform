//! K21 (R-209): a backtest is the simulation of its own weight schedule.

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
use pt_engine::schedule::{run_backtest, weights_schedule};
use pt_engine::simulate::{simulate, SimConfig};
use pt_portfolio::weights::SizingParams;
use pt_risk::pretrade::RiskLimits;
use pt_strategy::error::StrategyError;
use pt_strategy::estimate::VolParams;
use pt_strategy::strategy::Strategy;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

const SYMBOLS: [&str; 5] = ["A", "B", "C", "D", "E"];
const SESSIONS: usize = 400;

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
fn view(dates: &[NaiveDate]) -> MemoryView {
    let mut v = MemoryView::new();
    let mut state: u64 = 12345;
    for (k, s) in SYMBOLS.iter().enumerate() {
        let mut px: i64 = 100 + 10 * k as i64;
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
fn backtest_equals_simulation_of_its_weight_schedule() {
    // R-209
    let dates = sessions();
    let v = view(&dates);
    let cal = TradingCalendar::new(dates.clone()).unwrap();
    let universe: Vec<Symbol> = SYMBOLS.iter().map(|s| sym(s)).collect();
    let shortable: BTreeSet<Symbol> = universe.iter().cloned().collect();
    let cfg = SimConfig {
        start: dates[0],
        end: dates[SESSIONS - 1],
        initial_capital: dec(1_000_000),
        cost: CostSpec::zero(),
        limits: params().limits,
        universe: universe.clone(),
    };
    let schedule = rebalances(&dates);
    let p = params();

    let weights = weights_schedule(&schedule, &universe, &shortable, &AllLong, &v, &p).unwrap();
    let expected = simulate(&cfg, &weights, &v, &cal).unwrap();
    let actual = run_backtest(&cfg, &schedule, &shortable, &AllLong, &v, &cal, &p).unwrap();

    assert_eq!(actual.equity.len(), SESSIONS);
    assert_eq!(actual.equity, expected.equity);
    assert!(!actual.fills.is_empty(), "the schedule must trade");
    assert_eq!(actual.fills.len(), expected.fills.len());
    assert_eq!(actual.rejected.len(), expected.rejected.len());
    assert_eq!(actual.traded_notional, expected.traded_notional);
    let first = actual.equity.first().unwrap().1;
    assert!(actual.equity.iter().any(|(_, e)| *e != first));
}
