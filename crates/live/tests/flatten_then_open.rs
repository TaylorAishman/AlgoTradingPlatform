//! R-491: flattening before the first reconciliation leaves the paper book able to start clean.

mod common;

use common::LiveFixture;
use pt_alpaca::account;
use pt_gate::owner::Stage;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_live::settle::settle;
use pt_store::trading::Book;

const SPY_ID: &str = "pt-flatten-20240301-SPY";

// R-491
#[test]
fn flatten_fill_settle_then_reconcile_opens_a_clean_book_with_no_fills() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("SPY", 10);
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());

    flatten(&fx.ctx()).expect("flatten");
    fx.mock.fill(SPY_ID, 10, "141");
    settle(&fx.ctx()).expect("settle");

    let halts = fx.store.active_halts().expect("halts");
    let flattened = halts
        .iter()
        .find(|h| h.reason == "flattened")
        .expect("flattened halt");
    fx.store
        .clear_halt(flattened.id, "account flat", fx.now())
        .expect("clear");

    let report = reconcile(&fx.ctx()).expect("reconcile");

    assert!(report.clean, "reason {:?}", report.halt_reason);
    assert!(fx.store.fills().expect("fills").is_empty());
    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1, "ledger was {ledger:?}");
    assert_eq!(ledger[0].kind, "open_balance");
    let mock_cash = account::account(&fx.http).expect("account").cash;
    assert_eq!(ledger[0].cash_delta, mock_cash);
    assert!(fx
        .store
        .positions(Book::Paper)
        .expect("positions")
        .is_empty());
}
