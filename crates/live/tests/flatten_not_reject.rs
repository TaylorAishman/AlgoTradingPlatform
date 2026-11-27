//! R-489: an error that is not a broker reject stops flattening and leaves the order an intent.

mod common;

use common::{d, sym, LiveFixture};
use pt_alpaca::error::AlpacaError;
use pt_core::order_state::OrderState;
use pt_gate::owner::Stage;
use pt_live::error::LiveError;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_store::trading::{Book, LedgerEntry};
use rust_decimal::Decimal;

const SPY_ID: &str = "pt-flatten-20240301-SPY";

/// Ledger and mock position SPY +10, reconciled, stage `Paper`.
fn reconciled_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    fx.store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: d(2024, 3, 1),
            kind: "fill".to_string(),
            symbol: Some(sym("SPY")),
            qty: 10,
            price: None,
            cash_delta: Decimal::ZERO,
            ref_id: None,
        })
        .expect("ledger SPY");
    fx.mock.seed_position("SPY", 10);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    fx
}

// R-489
#[test]
fn a_401_on_the_order_post_stops_flatten_with_an_alpaca_error() {
    let fx = reconciled_fixture();
    fx.mock.reject_next("POST", "/v2/orders", 1, 401);

    let result = flatten(&fx.ctx());

    assert!(
        matches!(
            result,
            Err(LiveError::Alpaca(AlpacaError::Rejected { status: 401, .. }))
        ),
        "got {result:?}"
    );
}

// R-489
#[test]
fn a_401_leaves_the_flatten_row_an_intent_with_no_order_rejected_alert() {
    let fx = reconciled_fixture();
    fx.mock.reject_next("POST", "/v2/orders", 1, 401);

    let _ = flatten(&fx.ctx());

    let row = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(row.state, OrderState::Intent);
    let alerts = fx.store.alerts().expect("alerts");
    assert!(
        alerts
            .iter()
            .all(|(_, code, _, _)| code != "order_rejected"),
        "alerts: {alerts:?}"
    );
}

// R-489
#[test]
fn a_401_leaves_the_flattened_halt_active() {
    let fx = reconciled_fixture();
    fx.mock.reject_next("POST", "/v2/orders", 1, 401);

    let _ = flatten(&fx.ctx());

    let halts = fx.store.active_halts().expect("halts");
    assert!(halts.iter().any(|h| h.reason == "flattened"), "{halts:?}");
}
