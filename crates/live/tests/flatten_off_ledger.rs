//! R-490: a flatten order sent before the paper ledger exists is marked off-ledger.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_store::trading::Book;

const SPY_ID: &str = "pt-flatten-20240301-SPY";
const TLT_ID: &str = "pt-flatten-20240301-TLT";

// R-490
#[test]
fn flatten_with_an_empty_ledger_marks_every_flatten_order_off_ledger() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("SPY", 10);
    fx.mock.seed_position("TLT", -5);
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());

    flatten(&fx.ctx()).expect("flatten");

    for id in [SPY_ID, TLT_ID] {
        assert!(fx.store.order(id).expect("read").is_some(), "{id} stored");
        assert!(
            fx.store.is_off_ledger(id).expect("flag"),
            "{id} should be off-ledger"
        );
    }
}

// R-490
#[test]
fn flatten_with_an_opening_balance_leaves_every_flatten_order_on_ledger() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    assert!(!fx.store.ledger(Book::Paper).expect("ledger").is_empty());
    fx.mock.seed_position("SPY", 10);
    fx.mock.seed_position("TLT", -5);

    flatten(&fx.ctx()).expect("flatten");

    for id in [SPY_ID, TLT_ID] {
        assert!(fx.store.order(id).expect("read").is_some(), "{id} stored");
        assert!(
            !fx.store.is_off_ledger(id).expect("flag"),
            "{id} should be on-ledger"
        );
    }
}
