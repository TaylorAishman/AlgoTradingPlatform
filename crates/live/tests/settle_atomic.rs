//! R-475: settlement records each fill together with its ledger entry.

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

const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";
const TLT_ID: &str = "pt-tsmom-v1-20240229-TLT-1";

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

fn fixture(orders: &[(&str, &str, Side)]) -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    for (id, symbol, side) in orders {
        fx.store
            .insert_order(&intent(id, symbol, *side))
            .expect("insert");
    }
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    fx
}

fn fill_entries(fx: &LiveFixture) -> usize {
    fx.store
        .ledger(Book::Paper)
        .expect("ledger")
        .iter()
        .filter(|e| e.kind == "fill")
        .count()
}

// R-475
#[test]
fn a_full_fill_gives_one_fill_row_and_one_fill_ledger_entry() {
    let fx = fixture(&[(SPY_ID, "SPY", Side::Buy)]);
    fx.mock.fill(SPY_ID, 10, "141");

    settle(&fx.ctx()).expect("settle");

    assert_eq!(fx.store.fills().expect("fills").len(), 1);
    let ledger = fx.store.ledger(Book::Paper).expect("ledger");
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, "fill");
    assert_eq!(ledger[0].symbol, Some(sym("SPY")));
    assert_eq!(ledger[0].qty, 10);
    assert_eq!(ledger[0].cash_delta, dec("-1410"));
}

// R-475
#[test]
fn a_second_settle_adds_neither_a_fill_row_nor_a_ledger_entry() {
    let fx = fixture(&[(SPY_ID, "SPY", Side::Buy)]);
    fx.mock.fill(SPY_ID, 10, "141");
    settle(&fx.ctx()).expect("first settle");

    settle(&fx.ctx()).expect("second settle");

    assert_eq!(fx.store.fills().expect("fills").len(), 1);
    assert_eq!(fill_entries(&fx), 1);
}

// R-475
#[test]
fn the_ledger_fill_entries_equal_the_fill_rows_after_several_fills_and_settles() {
    let fx = fixture(&[(SPY_ID, "SPY", Side::Buy), (TLT_ID, "TLT", Side::Sell)]);
    fx.mock.fill(SPY_ID, 4, "141");
    fx.mock.fill(TLT_ID, 10, "90.5");
    settle(&fx.ctx()).expect("first settle");
    fx.mock.fill(SPY_ID, 6, "142");
    settle(&fx.ctx()).expect("second settle");
    settle(&fx.ctx()).expect("third settle");

    let fills = fx.store.fills().expect("fills");
    assert_eq!(fills.len(), 3);
    assert_eq!(fill_entries(&fx), fills.len());
    // Buy 4 at 141 and 6 at 142, sell 10 at 90.5.
    assert_eq!(
        fx.store.cash(Book::Paper).expect("cash"),
        dec("-564") - dec("852") + dec("905")
    );
}
