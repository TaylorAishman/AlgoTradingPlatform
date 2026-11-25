//! R-152: the strategy identifies itself and its parameters stably.

use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::types::Symbol;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use std::sync::Arc;

fn tsmom(lookbacks: Vec<u32>, rf: &str) -> Tsmom {
    let day = NaiveDate::from_ymd_opt(2024, 3, 28).expect("valid date");
    let cal = TradingCalendar::new(vec![day]).expect("ascending");
    Tsmom::new(
        TsmomParams {
            lookback_months: lookbacks,
            risk_free_symbol: Symbol::new(rf).expect("valid symbol"),
        },
        Arc::new(cal),
    )
}

#[test]
fn id_is_tsmom_v1() {
    // R-152
    assert_eq!(tsmom(vec![12], "BIL").id(), "tsmom-v1");
}

#[test]
fn params_canonical_is_identical_on_every_call() {
    // R-152
    let t = tsmom(vec![12], "BIL");
    assert_eq!(t.params_canonical(), t.params_canonical());
    assert_eq!(
        t.params_canonical(),
        tsmom(vec![12], "BIL").params_canonical()
    );
}

#[test]
fn params_canonical_differs_for_a_different_lookback_set() {
    // R-152
    assert_ne!(
        tsmom(vec![12], "BIL").params_canonical(),
        tsmom(vec![1, 3, 12], "BIL").params_canonical()
    );
}

#[test]
fn params_canonical_contains_the_risk_free_symbol() {
    // R-152
    assert!(tsmom(vec![12], "BIL").params_canonical().contains("BIL"));
}
