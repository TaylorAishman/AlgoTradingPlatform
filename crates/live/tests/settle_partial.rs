//! R-290: partial fills accumulate.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::settle::settle;
use pt_live::submit::submit_planned;
use pt_store::trading::{Book, OrderRow};
use rust_decimal::Decimal;
use std::str::FromStr;

const TLT_ID: &str = "pt-tsmom-v1-20240229-TLT-1";
const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("decimal")
}

fn intent(id: &str, symbol: &str, side: Side) -> OrderRow {
    OrderRow {
        client_order_id: id.to_string(),
        decision_id: None,
        symbol: sym(symbol),
        side,
        qty: 10,
        leg: 1,
        state: OrderState::Intent,
        broker_order_id: None,
        filled_qty: 0,
        reference_price: Decimal::from(100),
        trade_date: d(2024, 3, 1),
    }
}

fn auth() -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 3, 1),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    }
}

/// The R-283 setup: a TLT sell and an SPY buy, both submitted.
fn submitted_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(TLT_ID, "TLT", Side::Sell))
        .expect("insert TLT");
    fx.store
        .insert_order(&intent(SPY_ID, "SPY", Side::Buy))
        .expect("insert SPY");
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    fx
}

// R-290
#[test]
fn two_partial_fills_accumulate_to_a_filled_order() {
    let fx = submitted_fixture();

    fx.mock.fill(SPY_ID, 4, "141");
    let first = settle(&fx.ctx()).expect("first settle");

    assert_eq!(first.fills_recorded, 1);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::PartiallyFilled);
    assert_eq!(spy.filled_qty, 4);
    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.get(&sym("SPY")), Some(&4));
    assert_eq!(fx.store.cash(Book::Paper).expect("cash"), dec("-564"));

    fx.mock.fill(SPY_ID, 6, "142");
    let second = settle(&fx.ctx()).expect("second settle");

    assert_eq!(second.fills_recorded, 1);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Filled);
    assert_eq!(spy.filled_qty, 10);
    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.get(&sym("SPY")), Some(&10));
    assert_eq!(fx.store.cash(Book::Paper).expect("cash"), dec("-1416"));

    let fills = fx.store.fills().expect("fills");
    assert_eq!(fills.len(), 2);
    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 2);
    let mut deltas: Vec<Decimal> = ledger.iter().map(|e| e.cash_delta).collect();
    deltas.sort();
    assert_eq!(deltas, vec![dec("-852"), dec("-564")]);
    let mut prices: Vec<Decimal> = ledger.iter().filter_map(|e| e.price).collect();
    prices.sort();
    assert_eq!(prices, vec![dec("141"), dec("142")]);
    assert!(ledger.iter().all(|e| e.qty == 4 || e.qty == 6));
}

// R-290
#[test]
fn settling_again_after_both_partial_fills_changes_nothing() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");
    settle(&fx.ctx()).expect("first settle");
    fx.mock.fill(SPY_ID, 6, "142");
    settle(&fx.ctx()).expect("second settle");

    let report = settle(&fx.ctx()).expect("third settle");

    assert_eq!(report.fills_recorded, 0);
    assert_eq!(report.orders_updated, 0);
    assert_eq!(fx.store.fills().expect("fills").len(), 2);
    assert_eq!(fx.store.ledger(Book::Paper).expect("ledger").len(), 2);
    assert_eq!(fx.store.cash(Book::Paper).expect("cash"), dec("-1416"));
}

// R-290
#[test]
fn a_repeated_settle_between_the_partial_fills_does_not_double_count() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 4, "141");
    settle(&fx.ctx()).expect("first settle");

    let again = settle(&fx.ctx()).expect("repeat settle");

    assert_eq!(again.fills_recorded, 0);
    assert_eq!(again.orders_updated, 0);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::PartiallyFilled);
    assert_eq!(spy.filled_qty, 4);
    assert_eq!(fx.store.cash(Book::Paper).expect("cash"), dec("-564"));
}
