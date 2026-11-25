//! R-298: a position that differs from the ledger halts the system.

mod common;

use common::{d, sym, LiveFixture};
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;
use pt_store::trading::{Book, LedgerEntry};

fn fill(symbol: &str, qty: i64) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: d(2024, 3, 1),
        kind: "fill".to_string(),
        symbol: Some(sym(symbol)),
        qty,
        price: None,
        cash_delta: rust_decimal::Decimal::ZERO,
        ref_id: None,
    }
}

// R-298
#[test]
fn broker_position_absent_from_ledger_halts_naming_symbol_and_both_quantities() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);

    fx.mock.seed_position("GLD", 3);
    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(!report.clean);
    assert_eq!(
        report.halt_reason.as_deref(),
        Some("reconciliation_position")
    );

    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert_eq!(halts[0].reason, "reconciliation_position");
    let detail = &halts[0].detail;
    assert!(detail.contains("GLD"), "detail was {detail:?}");
    assert!(detail.contains('3'), "detail was {detail:?}");
    assert!(detail.contains('0'), "detail was {detail:?}");
    assert_eq!(detail, "GLD: broker 3, ledger 0");
}

// R-298
#[test]
fn ledger_position_missing_at_broker_halts() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    fx.store.append_ledger(&fill("SPY", 5)).expect("ledger");

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(!report.clean);
    assert_eq!(
        report.halt_reason.as_deref(),
        Some("reconciliation_position")
    );
    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert!(halts[0].detail.contains("SPY"), "{:?}", halts[0].detail);
    assert!(halts[0].detail.contains('5'), "{:?}", halts[0].detail);
}

// R-298
#[test]
fn quantity_mismatch_halts() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    fx.store.append_ledger(&fill("SPY", 5)).expect("ledger");
    fx.mock.seed_position("SPY", 7);

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert_eq!(
        report.halt_reason.as_deref(),
        Some("reconciliation_position")
    );
}

// R-298
#[test]
fn matching_positions_stay_clean() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    fx.store.append_ledger(&fill("SPY", 5)).expect("ledger");
    fx.mock.seed_position("SPY", 5);

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(report.clean, "reason {:?}", report.halt_reason);
    assert!(fx.store.active_halts().expect("halts").is_empty());
}

// R-298, R-315
#[test]
fn reconciling_twice_on_a_position_break_halts_and_alerts_once() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);

    fx.mock.seed_position("GLD", 3);
    reconcile(&fx.ctx()).expect("first");
    let report = reconcile(&fx.ctx()).expect("second");
    assert_eq!(
        report.halt_reason.as_deref(),
        Some("reconciliation_position")
    );

    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);
    let alerts = fx.store.alerts().expect("alerts");
    let n = alerts
        .iter()
        .filter(|(level, code, _, _)| level == "critical" && code == "halt_set")
        .count();
    assert_eq!(n, 1, "alerts were {alerts:?}");
}
