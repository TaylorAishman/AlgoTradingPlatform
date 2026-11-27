//! R-505: order and snapshot alerts are recorded at level `warn`.

mod common;

use chrono::NaiveDate;
use common::{d, sym, LiveFixture};
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar, Side};
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::settle::settle;
use pt_live::snapshot::snapshot_equity;
use pt_live::submit::submit_planned;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, LedgerEntry, OrderRow};
use rust_decimal::Decimal;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("decimal")
}

fn intent(id: &str, symbol: &str, side: Side, trade_date: NaiveDate) -> OrderRow {
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
        trade_date,
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

/// The levels of every alert with this code.
fn levels(fx: &LiveFixture, code: &str) -> Vec<String> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == code)
        .map(|a| a.0)
        .collect()
}

// R-505
#[test]
fn order_rejected_is_recorded_at_warn() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_asset("TLT", true, false, false);
    fx.store
        .insert_order(&intent(
            "pt-tsmom-v1-20240301-TLT-1",
            "TLT",
            Side::Sell,
            d(2024, 3, 1),
        ))
        .expect("insert");

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(levels(&fx, "order_rejected"), ["warn"]);
}

// R-505
#[test]
fn order_abandoned_for_a_passed_session_is_recorded_at_warn() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .insert_order(&intent(
            "pt-tsmom-v1-20240226-SPY-1",
            "SPY",
            Side::Buy,
            d(2024, 2, 27),
        ))
        .expect("insert");

    submit_planned(&fx.ctx(), &auth()).expect("submit");

    assert_eq!(levels(&fx, "order_abandoned"), ["warn"]);
}

// R-505
#[test]
fn incomplete_fill_is_recorded_at_warn() {
    let fx = LiveFixture::new(Stage::Paper);
    let id = "pt-tsmom-v1-20240229-SPY-1";
    fx.store
        .insert_order(&intent(id, "SPY", Side::Buy, d(2024, 3, 1)))
        .expect("insert");
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    fx.mock.fill(id, 4, "141");
    fx.mock.expire(id);

    settle(&fx.ctx()).expect("settle");

    assert_eq!(levels(&fx, "incomplete_fill"), ["warn"]);
}

// R-505
#[test]
fn missed_run_is_recorded_at_warn() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.store
        .append_ledger(&LedgerEntry {
            book: Book::Paper,
            date: d(2024, 2, 29),
            kind: "opening".to_string(),
            symbol: None,
            qty: 0,
            price: None,
            cash_delta: dec("100000"),
            ref_id: None,
        })
        .expect("opening");
    fx.store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date: d(2024, 3, 1),
            cash: dec("100000"),
            positions_value: dec("0"),
            equity: dec("100000"),
            recorded_at: fx.now(),
        })
        .expect("snapshot");
    let rows: Vec<BarRow> = ["2024-03-04", "2024-03-05", "2024-03-06"]
        .iter()
        .map(|dt| {
            let p = Price::new(dec("144")).expect("price");
            BarRow {
                bar: Bar {
                    symbol: sym("SPY"),
                    date: dt.parse().expect("date"),
                    open: p,
                    high: p,
                    low: p,
                    close: p,
                    volume: 1_000,
                },
                adjustment: Adjustment::Raw,
                feed: "sip".to_string(),
            }
        })
        .collect();
    fx.store.upsert_bars(&rows, fx.now()).expect("bars");

    let n = snapshot_equity(&fx.ctx(), d(2024, 3, 6)).expect("snapshot");

    assert_eq!(n, 3);
    assert_eq!(levels(&fx, "missed_run"), ["warn"]);
}
