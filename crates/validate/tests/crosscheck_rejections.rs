//! V08 (R-372): a run with rejected orders is not cross-checked.

mod common;

use common::{config, request, store};
use pt_validate::crosscheck::crosscheck;
use pt_validate::error::ValidateError;
use rust_decimal::Decimal;

fn cent() -> Decimal {
    Decimal::new(1, 2)
}

#[test]
fn rejected_orders_make_the_run_not_comparable() {
    // R-372: with risk.max_order_notional = 100 the orders are rejected.
    let mut cfg = config();
    cfg.risk.max_order_notional = Decimal::new(100, 0);
    let err = crosscheck(&store(), &cfg, &request(), true, cent()).expect_err("not comparable");
    assert!(matches!(err, ValidateError::NotComparable(_)), "{err:?}");
}

#[test]
fn rejected_orders_are_not_comparable_with_costs_either() {
    // R-372: the rule does not depend on the cost profile.
    let mut cfg = config();
    cfg.risk.max_order_notional = Decimal::new(100, 0);
    let err = crosscheck(&store(), &cfg, &request(), false, cent()).expect_err("not comparable");
    assert!(matches!(err, ValidateError::NotComparable(_)), "{err:?}");
}

#[test]
fn a_run_without_rejections_is_still_cross_checked() {
    // R-372: the default limit rejects nothing, so the check runs.
    let out = crosscheck(&store(), &config(), &request(), true, cent()).expect("crosscheck");
    assert!(out.equal);
}
