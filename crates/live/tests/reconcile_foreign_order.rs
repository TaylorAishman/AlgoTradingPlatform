//! R-297: an order the system did not create halts it.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;

// R-297
#[test]
fn foreign_order_halts_with_foreign_order_naming_it() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);

    fx.mock.seed_foreign_order("manual-1", "SPY", "buy", 1);
    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(!report.clean);
    assert_eq!(report.halt_reason.as_deref(), Some("foreign_order"));

    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert_eq!(halts[0].reason, "foreign_order");
    assert!(
        halts[0].detail.contains("manual-1"),
        "detail was {:?}",
        halts[0].detail
    );
}

// R-297
#[test]
fn foreign_order_records_a_critical_halt_set_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);

    fx.mock.seed_foreign_order("manual-1", "SPY", "buy", 1);
    reconcile(&fx.ctx()).expect("reconcile");

    let alerts = fx.store.alerts().expect("alerts");
    assert!(
        alerts
            .iter()
            .any(|(level, code, _, _)| level == "critical" && code == "halt_set"),
        "alerts were {alerts:?}"
    );
}

// R-297
#[test]
fn without_a_foreign_order_reconcile_stays_clean() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    let report = reconcile(&fx.ctx()).expect("again");
    assert!(report.clean);
    assert!(fx.store.active_halts().expect("halts").is_empty());
}

// R-297, R-315
#[test]
fn reconciling_twice_on_a_foreign_order_halts_and_alerts_once() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);

    fx.mock.seed_foreign_order("manual-1", "SPY", "buy", 1);
    reconcile(&fx.ctx()).expect("first");
    let report = reconcile(&fx.ctx()).expect("second");
    assert_eq!(report.halt_reason.as_deref(), Some("foreign_order"));

    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);
    let alerts = fx.store.alerts().expect("alerts");
    let n = alerts
        .iter()
        .filter(|(level, code, _, _)| level == "critical" && code == "halt_set")
        .count();
    assert_eq!(n, 1, "alerts were {alerts:?}");
}
