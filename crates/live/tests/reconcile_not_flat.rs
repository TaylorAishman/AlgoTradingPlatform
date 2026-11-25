//! R-296: a non-flat account halts the system before the first order.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;
use pt_store::trading::Book;

// R-296
#[test]
fn account_holding_a_position_halts_with_account_not_flat() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("GLD", 3);

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(!report.clean);
    assert_eq!(report.halt_reason.as_deref(), Some("account_not_flat"));

    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert_eq!(halts[0].reason, "account_not_flat");
}

// R-296
#[test]
fn non_flat_account_writes_no_ledger_entry() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("GLD", 3);

    reconcile(&fx.ctx()).expect("reconcile");
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());
}

// R-296, R-315
#[test]
fn reconciling_twice_on_a_not_flat_account_halts_and_alerts_once() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("GLD", 3);

    reconcile(&fx.ctx()).expect("first");
    let report = reconcile(&fx.ctx()).expect("second");
    assert_eq!(report.halt_reason.as_deref(), Some("account_not_flat"));

    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);
    let alerts = fx.store.alerts().expect("alerts");
    let n = alerts
        .iter()
        .filter(|(level, code, _, _)| level == "critical" && code == "halt_set")
        .count();
    assert_eq!(n, 1, "alerts were {alerts:?}");
}
