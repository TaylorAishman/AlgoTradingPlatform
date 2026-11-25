//! R-299: cash that differs from the ledger by more than the tolerance halts the system.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;

fn opened(fx: &LiveFixture) {
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
}

// R-299
#[test]
fn cash_one_cent_off_stays_clean() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_account("100000.01");

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(report.clean, "reason {:?}", report.halt_reason);
    assert_eq!(report.halt_reason, None);
    assert!(fx.store.active_halts().expect("halts").is_empty());
}

// R-299
#[test]
fn cash_two_cents_off_halts_with_reconciliation_cash() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_account("100000.02");

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(!report.clean);
    assert_eq!(report.halt_reason.as_deref(), Some("reconciliation_cash"));
    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert_eq!(halts[0].reason, "reconciliation_cash");
}

// R-299
#[test]
fn cash_two_cents_below_halts_too() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_account("99999.98");

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert_eq!(report.halt_reason.as_deref(), Some("reconciliation_cash"));
}

// R-299
#[test]
fn cash_one_cent_below_stays_clean() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_account("99999.99");

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(report.clean, "reason {:?}", report.halt_reason);
}

// R-299, R-315
#[test]
fn reconciling_twice_on_a_cash_break_halts_and_alerts_once() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_account("100000.02");

    reconcile(&fx.ctx()).expect("first");
    let report = reconcile(&fx.ctx()).expect("second");
    assert_eq!(report.halt_reason.as_deref(), Some("reconciliation_cash"));

    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);
    let alerts = fx.store.alerts().expect("alerts");
    let n = alerts
        .iter()
        .filter(|(level, code, _, _)| level == "critical" && code == "halt_set")
        .count();
    assert_eq!(n, 1, "alerts were {alerts:?}");
}
