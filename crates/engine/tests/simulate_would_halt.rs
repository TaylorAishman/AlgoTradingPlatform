mod common;

use chrono::NaiveDate;
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::{Side, Weights};
use pt_core::view::MemoryView;
use pt_engine::simulate::{simulate, SimConfig};
use pt_risk::halts::HaltReason;
use pt_risk::pretrade::RiskLimits;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn config(daily_loss_limit: &str, max_drawdown: &str) -> SimConfig {
    SimConfig {
        start: d(2024, 1, 2),
        end: d(2024, 1, 5),
        initial_capital: dec(100000),
        cost: CostSpec::zero(),
        limits: RiskLimits {
            max_order_notional: dec(10000000),
            max_weight: dec(1),
            max_gross: dec(1),
            daily_loss_limit: dd(daily_loss_limit),
            max_drawdown: dd(max_drawdown),
        },
        universe: vec![sym("A")],
    }
}

fn calendar() -> TradingCalendar {
    TradingCalendar::new(vec![
        d(2024, 1, 2),
        d(2024, 1, 3),
        d(2024, 1, 4),
        d(2024, 1, 5),
        d(2024, 1, 8),
    ])
    .unwrap()
}

/// Long A all in on 01-03, then A closes 5% down on 01-04 and stays there.
fn view(close_on_4: i64) -> MemoryView {
    let mut v = MemoryView::new();
    v.push_exec(bar("A", d(2024, 1, 2), 100, 100));
    v.push_exec(bar("A", d(2024, 1, 3), 100, 100));
    v.push_exec(bar("A", d(2024, 1, 4), 100, close_on_4));
    v.push_exec(bar("A", d(2024, 1, 5), close_on_4, close_on_4));
    v
}

fn weights() -> BTreeMap<NaiveDate, Weights> {
    let mut w1 = Weights::new();
    w1.insert(sym("A"), dd("1"));
    let mut w2 = Weights::new();
    w2.insert(sym("A"), dd("0.5"));
    BTreeMap::from([(d(2024, 1, 3), w1), (d(2024, 1, 5), w2)])
}

// R-206
#[test]
fn a_five_percent_fall_in_one_session_lists_that_date_as_daily_loss() {
    let r = simulate(&config("0.03", "0.15"), &weights(), &view(95), &calendar()).unwrap();
    assert_eq!(r.equity[2], (d(2024, 1, 4), dec(95000)));
    assert_eq!(r.would_halt, vec![(d(2024, 1, 4), HaltReason::DailyLoss)]);
}

// R-206
#[test]
fn the_run_still_trades_on_the_trade_date_after_the_halt() {
    let r = simulate(&config("0.03", "0.15"), &weights(), &view(95), &calendar()).unwrap();
    let sold = r
        .fills
        .iter()
        .any(|(date, f)| *date == d(2024, 1, 5) && f.side == Side::Sell && f.symbol == sym("A"));
    assert!(sold, "{:?}", r.fills);
}

// R-206
#[test]
fn a_halt_is_not_a_rejection() {
    let r = simulate(&config("0.03", "0.15"), &weights(), &view(95), &calendar()).unwrap();
    assert!(r.clean());
}

// R-206
#[test]
fn no_fall_beyond_the_limits_records_no_halt() {
    let r = simulate(&config("0.03", "0.15"), &weights(), &view(100), &calendar()).unwrap();
    assert!(r.would_halt.is_empty(), "{:?}", r.would_halt);
}

// R-206
#[test]
fn a_drawdown_past_the_limit_lists_the_date_as_drawdown() {
    let r = simulate(&config("0.5", "0.04"), &weights(), &view(95), &calendar()).unwrap();
    assert!(
        r.would_halt
            .contains(&(d(2024, 1, 4), HaltReason::Drawdown)),
        "{:?}",
        r.would_halt
    );
}
