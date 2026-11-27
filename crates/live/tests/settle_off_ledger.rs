//! R-479: a fill of an off-ledger order updates the order and stays out of the
//! fills and the ledger.

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

const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

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

// R-479
#[test]
fn a_full_fill_of_an_off_ledger_order_updates_the_row_but_not_fills_or_ledger() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&OrderRow {
            client_order_id: SPY_ID.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Buy,
            qty: 10,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::from(100),
            trade_date: d(2024, 3, 1),
        })
        .expect("insert");
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    fx.store.mark_off_ledger(SPY_ID).expect("mark off ledger");
    fx.mock.fill(SPY_ID, 10, "141");

    let report = settle(&fx.ctx()).expect("settle");

    assert_eq!(report.orders_updated, 1);
    assert_eq!(report.fills_recorded, 0);
    let row = fx.store.order(SPY_ID).expect("read").expect("row");
    assert_eq!(row.state, OrderState::Filled);
    assert_eq!(row.filled_qty, row.qty);
    assert!(fx.store.fills().expect("fills").is_empty());
    assert!(fx.store.ledger(Book::Paper).expect("ledger").is_empty());
}
