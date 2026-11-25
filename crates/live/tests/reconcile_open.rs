//! R-295: the first reconciliation records the opening balance of a flat account.

mod common;

use common::{d, LiveFixture};
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;
use pt_store::trading::Book;
use rust_decimal::Decimal;

// R-295
#[test]
fn first_reconcile_of_a_flat_account_records_the_opening_balance() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());

    let report = reconcile(&fx.ctx()).expect("reconcile");
    assert!(report.clean);
    assert_eq!(report.halt_reason, None);

    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, "open_balance");
    assert_eq!(ledger[0].cash_delta, Decimal::from(100000));
    assert_eq!(ledger[0].date, d(2024, 2, 29));
    assert_eq!(
        fx.store.cash(Book::Paper).expect("cash"),
        Decimal::from(100000)
    );
    assert!(fx.store.active_halts().expect("halts").is_empty());
}

// R-295
#[test]
fn second_reconcile_adds_nothing() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("first").clean);
    let report = reconcile(&fx.ctx()).expect("second");
    assert!(report.clean);
    assert_eq!(fx.store.ledger(Book::Paper).expect("ledger").len(), 1);
    assert_eq!(
        fx.store.cash(Book::Paper).expect("cash"),
        Decimal::from(100000)
    );
}
