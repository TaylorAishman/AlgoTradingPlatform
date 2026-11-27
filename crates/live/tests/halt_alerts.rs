//! R-474: every halt the system sets on its own raises one critical `halt_set` alert.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;
use pt_live::settle::settle;
use pt_live::snapshot::check_halts;
use pt_live::submit::submit_planned;
use pt_store::trading::{Book, EquitySnapshot, OrderRow};
use rust_decimal::Decimal;
use std::str::FromStr;

/// Asserts exactly one halt with `reason` and exactly one critical `halt_set` alert.
fn assert_one_halt_one_alert(fx: &LiveFixture, reason: &str) {
    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1, "halts were {halts:?}");
    assert_eq!(halts[0].reason, reason);
    let alerts = fx.store.alerts().expect("alerts");
    let halt_set: Vec<_> = alerts.iter().filter(|a| a.1 == "halt_set").collect();
    assert_eq!(halt_set.len(), 1, "alerts were {alerts:?}");
    assert_eq!(halt_set[0].0, "critical");
}

fn opened(fx: &LiveFixture) {
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
}

// R-474
#[test]
fn a_foreign_order_sets_one_halt_and_one_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_foreign_order("manual-1", "SPY", "buy", 1);
    reconcile(&fx.ctx()).expect("reconcile");
    assert_one_halt_one_alert(&fx, "foreign_order");
}

// R-474
#[test]
fn a_position_break_sets_one_halt_and_one_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_position("GLD", 3);
    reconcile(&fx.ctx()).expect("reconcile");
    assert_one_halt_one_alert(&fx, "reconciliation_position");
}

// R-474
#[test]
fn a_cash_break_sets_one_halt_and_one_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    opened(&fx);
    fx.mock.seed_account("100000.02");
    reconcile(&fx.ctx()).expect("reconcile");
    assert_one_halt_one_alert(&fx, "reconciliation_cash");
}

// R-474
#[test]
fn a_non_flat_first_reconcile_sets_one_halt_and_one_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_position("GLD", 3);
    reconcile(&fx.ctx()).expect("reconcile");
    assert_one_halt_one_alert(&fx, "account_not_flat");
}

// R-474
#[test]
fn an_unexpected_order_status_sets_one_halt_and_one_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    let id = "pt-tsmom-v1-20240229-SPY-1";
    fx.store
        .insert_order(&OrderRow {
            client_order_id: id.to_string(),
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
    let auth = AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 3, 1),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    };
    submit_planned(&fx.ctx(), &auth).expect("submit");
    fx.mock.set_order_status(id, "replaced");
    settle(&fx.ctx()).expect("settle");
    assert_one_halt_one_alert(&fx, "unexpected_order_status");
}

// R-474
#[test]
fn a_daily_loss_breach_sets_one_halt_and_one_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    for (date, equity) in [("2024-02-29", "100000"), ("2024-03-01", "96900")] {
        let e = Decimal::from_str(equity).expect("decimal");
        fx.store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date: date.parse().expect("date"),
                cash: e,
                positions_value: Decimal::ZERO,
                equity: e,
                recorded_at: fx.now(),
            })
            .expect("snapshot");
    }
    check_halts(&fx.ctx()).expect("check");
    assert_one_halt_one_alert(&fx, "daily_loss");
}
