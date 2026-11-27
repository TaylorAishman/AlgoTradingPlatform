//! R-488: a flatten retry adopts the earlier order if the broker holds it.

mod common;

use common::{d, sym, LiveFixture};
use pt_alpaca::orders_read::order_by_client_id;
use pt_core::order_state::OrderState;
use pt_gate::owner::Stage;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_store::trading::{Book, LedgerEntry};
use rust_decimal::Decimal;

const SPY_ID: &str = "pt-flatten-20240301-SPY";
const RETRY_ID: &str = "pt-flatten-20240301-SPY-r1";

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

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .count()
}

// R-488
#[test]
fn a_retry_adopts_the_order_the_broker_kept_and_sends_no_post() {
    let fx = reconciled_fixture();
    fx.mock.reject_but_keep_next_order(422);

    let first = flatten(&fx.ctx()).expect("first flatten");
    assert_eq!(first.rejected, 1);
    let posts_after_first = posts(&fx);

    let second = flatten(&fx.ctx()).expect("second flatten");

    assert_eq!(posts(&fx), posts_after_first, "the retry sent no POST");
    let broker_id = order_by_client_id(&fx.http, SPY_ID)
        .expect("lookup")
        .expect("broker holds the order")
        .id;
    let row = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(row.state, OrderState::Submitted);
    assert_eq!(row.broker_order_id.as_deref(), Some(broker_id.as_str()));
    assert!(fx.store.order(RETRY_ID).expect("read").is_none());
    assert_eq!(second.accepted, 1);
}

// R-488
#[test]
fn a_retry_after_a_reject_the_broker_did_not_keep_posts_a_new_order_with_suffix_r1() {
    let fx = reconciled_fixture();
    fx.mock.reject_next("POST", "/v2/orders", 1, 422);

    let first = flatten(&fx.ctx()).expect("first flatten");
    assert_eq!(first.rejected, 1);
    let posts_after_first = posts(&fx);

    flatten(&fx.ctx()).expect("second flatten");

    assert_eq!(posts(&fx), posts_after_first + 1, "the retry posted once");
    assert!(fx.store.order(RETRY_ID).expect("read").is_some());
}
