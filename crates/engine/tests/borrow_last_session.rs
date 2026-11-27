mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::money::Money;
use pt_core::types::Weights;
use pt_core::view::MemoryView;
use pt_engine::cost::borrow_cost;
use pt_engine::simulate::{simulate, SimConfig};
use pt_risk::pretrade::RiskLimits;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

fn dd(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn cost() -> CostSpec {
    CostSpec {
        spread_bps: dec(0),
        fee_bps: dec(0),
        impact_coeff: dec(0),
        borrow_bps_annual: dec(360),
    }
}

// R-455
#[test]
fn short_held_at_the_last_session_close_is_charged_one_day_not_the_weekend() {
    // 2024-01-04 is a Thursday and 01-05 a Friday, the calendar's last session.
    let (thu, fri) = (d(2024, 1, 4), d(2024, 1, 5));
    let mut view = MemoryView::new();
    view.push_exec(bar("B", thu, 50, 50));
    view.push_exec(bar("B", fri, 49, 48));
    let mut w = Weights::new();
    w.insert(sym("B"), dd("-0.25"));
    let weights = BTreeMap::from([(fri, w)]);
    let cfg = SimConfig {
        start: thu,
        end: fri,
        initial_capital: dec(100000),
        cost: cost(),
        limits: RiskLimits {
            max_order_notional: dec(1000000),
            max_weight: dec(1),
            max_gross: dec(1),
            daily_loss_limit: dd("0.03"),
            max_drawdown: dd("0.15"),
        },
        universe: vec![sym("B")],
    };
    let cal = TradingCalendar::new(vec![thu, fri]).unwrap();
    let r = simulate(&cfg, &weights, &view, &cal).unwrap();

    let short_qty: u64 = r.fills.iter().map(|(_, f)| f.qty).sum();
    assert!(short_qty > 0);
    let short_value = Money(dec(48) * Decimal::from(short_qty));
    let one_day = borrow_cost(&cost(), short_value, 1).0;
    assert!(one_day > Decimal::ZERO);
    assert_eq!(r.borrow_total, one_day);
}
