//! R-289: each fill is recorded once, with one ledger entry.

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

// R-289
#[test]
fn a_full_fill_is_recorded_with_one_fill_row_and_one_ledger_entry() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 10, "141");

    let report = settle(&fx.ctx()).expect("settle");

    assert_eq!(report.fills_recorded, 1);
    let fills = fx.store.fills().expect("fills");
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].symbol, sym("SPY"));
    assert_eq!(fills[0].side, Side::Buy);
    assert_eq!(fills[0].qty, 10);
    assert_eq!(fills[0].price, dec("141"));

    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.len(), 1);
    assert_eq!(positions.get(&sym("SPY")), Some(&10));

    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1);
    let entry = &ledger[0];
    assert_eq!(entry.kind, "fill");
    assert_eq!(entry.symbol, Some(sym("SPY")));
    assert_eq!(entry.qty, 10);
    assert_eq!(entry.price, Some(dec("141")));
    assert_eq!(entry.cash_delta, dec("-1410"));
    assert_eq!(entry.date, d(2024, 3, 1));
    assert_eq!(entry.date, fills[0].at.date_naive());
    assert_eq!(fx.store.cash(Book::Paper).expect("cash"), dec("-1410"));
}

// R-289
#[test]
fn a_second_settle_records_nothing_more() {
    let fx = submitted_fixture();
    fx.mock.fill(SPY_ID, 10, "141");
    settle(&fx.ctx()).expect("first settle");

    let report = settle(&fx.ctx()).expect("second settle");

    assert_eq!(report.fills_recorded, 0);
    assert_eq!(fx.store.fills().expect("fills").len(), 1);
    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].cash_delta, dec("-1410"));
    assert_eq!(fx.store.cash(Book::Paper).expect("cash"), dec("-1410"));
    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.get(&sym("SPY")), Some(&10));
}

// R-289
#[test]
fn a_sell_fill_adds_cash_and_reduces_the_position() {
    let fx = submitted_fixture();
    fx.mock.fill(TLT_ID, 10, "90.5");

    let report = settle(&fx.ctx()).expect("settle");

    assert_eq!(report.fills_recorded, 1);
    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].qty, -10);
    assert_eq!(ledger[0].price, Some(dec("90.5")));
    assert_eq!(ledger[0].cash_delta, dec("905"));
    let positions = fx.store.positions(Book::Paper).expect("positions");
    assert_eq!(positions.get(&sym("TLT")), Some(&-10));
}

// R-289
#[test]
fn an_order_with_no_fill_records_nothing() {
    let fx = submitted_fixture();

    let report = settle(&fx.ctx()).expect("settle");

    assert_eq!(report.fills_recorded, 0);
    assert!(fx.store.fills().expect("fills").is_empty());
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());
}

// R-289: the broker's order record can run ahead of its activity feed.
#[test]
fn an_order_reported_filled_before_its_fill_activity_stays_open_until_the_fill_arrives() {
    let fx = submitted_fixture();
    // The order endpoint says filled, but no FILL activity exists yet.
    fx.mock.set_order_status(SPY_ID, "filled");

    let report = settle(&fx.ctx()).expect("first settle");

    assert_eq!(report.fills_recorded, 0);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Submitted);
    assert_eq!(spy.filled_qty, 0);
    assert!(fx.store.fills().expect("fills").is_empty());
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());

    // The activity appears (the mock only scripts a fill on an open order).
    fx.mock.set_order_status(SPY_ID, "accepted");
    fx.mock.fill(SPY_ID, 10, "141");

    let report = settle(&fx.ctx()).expect("second settle");

    assert_eq!(report.fills_recorded, 1);
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Filled);
    assert_eq!(spy.filled_qty, 10);
    assert_eq!(fx.store.fills().expect("fills").len(), 1);
    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].cash_delta, dec("-1410"));
}
