//! V07 (R-371): the engine and the vector calculator agree within one cent with
//! proportional and borrow costs.

mod common;

use common::{config, date, request, store};
use pt_core::clock::FixedClock;
use pt_validate::crosscheck::crosscheck;
use pt_validate::runner::run_and_record;
use rust_decimal::Decimal;

fn cent() -> Decimal {
    Decimal::new(1, 2)
}

#[test]
fn research_costs_agree_within_one_cent() {
    // R-371
    let out = crosscheck(&store(), &config(), &request(), false, cent()).expect("crosscheck");
    assert!(out.equal);
    assert!(out.max_abs_diff <= cent());
    assert_eq!(out.first_diff_date, None);
}

#[test]
fn heavy_spread_fee_and_borrow_agree_within_one_cent() {
    // R-371: costs large enough that a wrong formula would show up as a difference.
    let mut cfg = config();
    cfg.cost.research.spread_bps = Decimal::new(20, 0);
    cfg.cost.research.fee_bps = Decimal::new(5, 0);
    cfg.cost.research.borrow_bps_annual = Decimal::new(5_000, 0);

    let clock = FixedClock(common::fetched_at());
    let run = run_and_record(&store(), &cfg, &request(), &clock).expect("run");
    assert!(
        run.result.cost_total > Decimal::ZERO,
        "no proportional cost"
    );
    assert!(run.result.borrow_total > Decimal::ZERO, "no borrow cost");

    let out = crosscheck(&store(), &cfg, &request(), false, cent()).expect("crosscheck");
    assert!(out.equal, "max diff {}", out.max_abs_diff);
    assert!(out.max_abs_diff <= cent());
}

#[test]
fn impact_coefficient_is_set_to_zero_for_both_calculators() {
    // R-371: the vector calculator has no impact term, so the check zeroes it.
    let mut cfg = config();
    cfg.cost.research.impact_coeff = Decimal::new(50, 0);
    let out = crosscheck(&store(), &cfg, &request(), false, cent()).expect("crosscheck");
    assert!(out.equal, "max diff {}", out.max_abs_diff);
}

#[test]
fn borrow_agrees_when_the_range_ends_on_the_last_session() {
    // R-371: borrow accrues for 1 day on the last session of the calendar.
    let mut cfg = config();
    cfg.cost.research.borrow_bps_annual = Decimal::new(5_000, 0);
    let mut req = request();
    req.to = date(2024, 12, 31);
    let out = crosscheck(&store(), &cfg, &req, false, cent()).expect("crosscheck");
    assert!(out.equal, "max diff {}", out.max_abs_diff);
}
