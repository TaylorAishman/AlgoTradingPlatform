//! R-322, R-487: `flatten` reports how many orders it cancelled, how many the
//! broker accepted and how many the broker rejected.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_gate::owner::Stage;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_store::trading::{Book, LedgerEntry};
use rust_decimal::Decimal;

const SPY_ID: &str = "pt-flatten-20240301-SPY";
const TLT_ID: &str = "pt-flatten-20240301-TLT";

fn fill(symbol: &str, qty: i64) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: d(2024, 3, 1),
        kind: "fill".to_string(),
        symbol: Some(sym(symbol)),
        qty,
        price: None,
        cash_delta: Decimal::ZERO,
        ref_id: None,
    }
}

/// Ledger and mock positions SPY +10 and TLT -5, reconciled, stage `Paper`.
fn reconciled_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    fx.store
        .append_ledger(&fill("SPY", 10))
        .expect("ledger SPY");
    fx.store
        .append_ledger(&fill("TLT", -5))
        .expect("ledger TLT");
    fx.mock.seed_position("SPY", 10);
    fx.mock.seed_position("TLT", -5);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    fx
}

// R-322
#[test]
fn flatten_while_halted_reports_no_cancels_two_accepted_and_no_rejects() {
    let fx = reconciled_fixture();
    fx.store
        .set_halt("manual", "halted for the test", fx.now())
        .expect("halt");

    let report = flatten(&fx.ctx()).expect("flatten while halted");

    assert_eq!(report.cancelled, 0);
    assert_eq!(report.accepted, 2);
    assert_eq!(report.rejected, 0);
}

// R-487
#[test]
fn flatten_counts_a_broker_rejected_order_as_rejected_and_the_other_as_accepted() {
    let fx = reconciled_fixture();
    fx.mock.reject_next("POST", "/v2/orders", 1, 422);

    let report = flatten(&fx.ctx()).expect("a rejection does not abort the flatten");

    assert_eq!(report.accepted, 1);
    assert_eq!(report.rejected, 1);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Rejected);
    let tlt = fx.store.order(TLT_ID).expect("read").expect("TLT row");
    assert_eq!(tlt.state, OrderState::Submitted);
    let alerts = fx.store.alerts().expect("alerts");
    let rejected = alerts
        .iter()
        .filter(|(_, code, _, _)| code == "order_rejected")
        .count();
    assert_eq!(rejected, 1, "alerts: {alerts:?}");
}
