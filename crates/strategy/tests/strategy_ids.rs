//! R-451: each strategy's id is available as a constant.

use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Symbol;
use pt_strategy::benchmark::{self, LongOnly};
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{self, Tsmom, TsmomParams};
use std::sync::Arc;

fn calendar() -> Arc<TradingCalendar> {
    let d = NaiveDate::from_ymd_opt(2024, 3, 28).expect("valid date");
    Arc::new(TradingCalendar::new(vec![d]).expect("ascending"))
}

#[test]
fn tsmom_id_constant_matches_the_strategy_id() {
    // R-451
    let s = Tsmom::new(
        TsmomParams {
            lookback_months: vec![12],
            risk_free_symbol: Symbol::new("R").expect("valid symbol"),
        },
        calendar(),
    );
    assert_eq!(tsmom::ID, "tsmom-v1");
    assert_eq!(s.id(), tsmom::ID);
}

#[test]
fn benchmark_id_constant_matches_the_strategy_id() {
    // R-451
    let s = LongOnly::new(calendar());
    assert_eq!(benchmark::ID, "bh-volscaled-v1");
    assert_eq!(s.id(), benchmark::ID);
}
