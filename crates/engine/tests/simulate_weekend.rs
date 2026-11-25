mod common;

use chrono::NaiveDate;
use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::types::Weights;
use pt_core::view::MemoryView;
use pt_engine::simulate::{simulate, SimConfig, SimResult};
use pt_risk::pretrade::RiskLimits;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn config(start: NaiveDate, end: NaiveDate) -> SimConfig {
    SimConfig {
        start,
        end,
        initial_capital: dec(100000),
        cost: CostSpec {
            spread_bps: dec(10),
            fee_bps: dec(0),
            impact_coeff: dec(0),
            borrow_bps_annual: dec(360),
        },
        limits: RiskLimits {
            max_order_notional: dec(1000000),
            max_weight: dec(1),
            max_gross: dec(1),
            daily_loss_limit: dd("0.03"),
            max_drawdown: dd("0.15"),
        },
        universe: vec![sym("A"), sym("B")],
    }
}

/// The worked example's bars and weights moved onto the three given sessions.
fn run(sessions: [NaiveDate; 3], calendar: Vec<NaiveDate>) -> SimResult {
    let mut view = MemoryView::new();
    for (s, day, open, close) in WORKED_BARS {
        view.push_exec(bar(s, sessions[(day - 2) as usize], open, close));
    }
    let mut w1 = Weights::new();
    w1.insert(sym("A"), dd("0.5"));
    w1.insert(sym("B"), dd("-0.25"));
    let mut w2 = Weights::new();
    w2.insert(sym("A"), dd("-0.2"));
    w2.insert(sym("B"), dd("0"));
    let weights = BTreeMap::from([(sessions[1], w1), (sessions[2], w2)]);
    let cal = TradingCalendar::new(calendar).unwrap();
    simulate(&config(sessions[0], sessions[2]), &weights, &view, &cal).unwrap()
}

// R-201
#[test]
fn friday_short_is_charged_three_days_of_borrow() {
    // 2024-01-04 is a Thursday, 01-05 a Friday, 01-08 a Monday.
    let r = run(
        [d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 8)],
        vec![d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 8), d(2024, 1, 9)],
    );
    assert_eq!(
        r.equity.iter().map(|&(_, e)| e).collect::<Vec<_>>(),
        vec![dec(100000), dd("100917.80"), dd("101623.4602")]
    );
}

// R-201
#[test]
fn weekend_adds_two_days_of_borrow_to_the_total() {
    let r = run(
        [d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 8)],
        vec![d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 8), d(2024, 1, 9)],
    );
    // 2.40 on Friday for 3 days, 2.0488 on Monday for 1 day.
    assert_eq!(r.borrow_total, dd("9.2488"));
}

// R-201
#[test]
fn holiday_gap_is_charged_by_calendar_days_to_the_next_session() {
    // Friday 01-05 to Tuesday 01-09: 4 calendar days, 9.60 of borrow.
    let r = run(
        [d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 9)],
        vec![d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 9)],
    );
    assert_eq!(r.equity[1], (d(2024, 1, 5), dd("100915.40")));
}

// R-201
#[test]
fn last_calendar_session_is_charged_one_day() {
    // Same dates as the weekend case but the calendar ends on 01-08: Monday
    // has no next session and is charged 1 day, so the total is unchanged.
    let r = run(
        [d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 8)],
        vec![d(2024, 1, 4), d(2024, 1, 5), d(2024, 1, 8)],
    );
    assert_eq!(r.borrow_total, dd("9.2488"));
}
