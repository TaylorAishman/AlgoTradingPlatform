//! V06 (R-370): the engine and the vector calculator agree exactly at zero cost.

mod common;

use common::{config, request, store};
use pt_core::clock::FixedClock;
use pt_core::types::Side;
use pt_validate::crosscheck::crosscheck;
use pt_validate::runner::run_and_record;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[test]
fn engine_and_vector_agree_exactly_at_zero_cost() {
    // R-370
    let out = crosscheck(&store(), &config(), &request(), true, Decimal::ZERO).expect("crosscheck");
    assert!(out.equal);
    assert_eq!(out.max_abs_diff, Decimal::ZERO);
    assert_eq!(out.first_diff_date, None);
}

#[test]
fn the_fixture_run_has_a_position_that_changes_sign() {
    // R-370: the agreement is only meaningful if the run goes long and short.
    let clock = FixedClock(common::fetched_at());
    let out = run_and_record(&store(), &config(), &request(), &clock).expect("run");
    let mut pos: BTreeMap<String, i64> = BTreeMap::new();
    let mut last_sign: BTreeMap<String, bool> = BTreeMap::new();
    let mut flipped = false;
    for (_, fill) in &out.result.fills {
        let qty = i64::try_from(fill.qty).expect("small qty");
        let signed = match fill.side {
            Side::Buy => qty,
            Side::Sell => -qty,
        };
        let p = pos.entry(fill.symbol.to_string()).or_insert(0);
        *p += signed;
        if *p != 0 {
            let long = *p > 0;
            if last_sign.insert(fill.symbol.to_string(), long) == Some(!long) {
                flipped = true;
            }
        }
    }
    assert!(flipped, "no position changed sign in the fixture run");
}
