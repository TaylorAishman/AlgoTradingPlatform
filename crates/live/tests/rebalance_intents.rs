//! R-303: the planned orders are written as Intent orders linked to the decision, nothing is
//! sent to the broker, and a second call for the same signal date is NotDue and writes nothing.

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar, Side};
use pt_data::reference::sync_assets;
use pt_gate::owner::Stage;
use pt_live::rebalance::{rebalance, RebalanceOutcome};
use pt_live::reconcile::reconcile;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, Mode};
use rust_decimal::Decimal;

fn price(x: f64) -> Price {
    Price::from_f64_shortest(x).expect("price")
}

/// The fixture's closes for the first 41 sessions (through 2024-02-29), for every symbol
/// except `skip` on the last of them.
fn seed_bars(fx: &LiveFixture, skip_last: Option<&str>) {
    let sessions = sessions_2024();
    let mut rows = Vec::new();
    let mut prev: [Option<f64>; 3] = [None; 3];
    for (i, date) in sessions.iter().take(41).enumerate() {
        let i = i as i64;
        let closes = [
            (100 + i + (i % 2)) as f64,
            (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0,
            (10000 + i) as f64 / 100.0,
        ];
        for (k, name) in ["SPY", "TLT", "BIL"].iter().enumerate() {
            let close = closes[k];
            let open = prev[k].unwrap_or(close);
            prev[k] = Some(close);
            if i == 40 && skip_last == Some(*name) {
                continue;
            }
            for adjustment in [Adjustment::Raw, Adjustment::All] {
                rows.push(BarRow {
                    bar: Bar {
                        symbol: sym(name),
                        date: *date,
                        open: price(open),
                        high: price(open.max(close)),
                        low: price(open.min(close)),
                        close: price(close),
                        volume: 1_000_000,
                    },
                    adjustment,
                    feed: "sip".to_string(),
                });
            }
        }
    }
    fx.store.upsert_bars(&rows, Utc::now()).expect("bars");
}

fn ready_paper_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    seed_bars(&fx, None);
    sync_assets(&fx.store, &fx.http, &fx.cfg, fx.now()).expect("assets");
    fx.store
        .put_equity_snapshot(&EquitySnapshot {
            book: Book::Paper,
            date: d(2024, 2, 29),
            cash: Decimal::from(100000),
            positions_value: Decimal::ZERO,
            equity: Decimal::from(100000),
            recorded_at: fx.now(),
        })
        .expect("snapshot");
    fx
}

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST")
        .count()
}

fn all_orders(fx: &LiveFixture) -> Vec<pt_store::trading::OrderRow> {
    let mut rows = fx
        .store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::DryRun,
        ])
        .expect("orders");
    rows.sort_by(|a, b| a.client_order_id.cmp(&b.client_order_id));
    rows
}

// R-303
#[test]
fn planned_orders_are_written_as_intents_for_spy_buy_and_tlt_sell() {
    let fx = ready_paper_fixture();
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    let RebalanceOutcome::Recorded {
        decision_id,
        orders,
    } = outcome
    else {
        panic!("expected Recorded, got {outcome:?}");
    };
    assert_eq!(orders, 2);

    let rows = all_orders(&fx);
    assert_eq!(rows.len(), 2);
    let spy = fx
        .store
        .order("pt-tsmom-v1-20240229-SPY-1")
        .expect("lookup")
        .expect("SPY intent exists");
    let tlt = fx
        .store
        .order("pt-tsmom-v1-20240229-TLT-1")
        .expect("lookup")
        .expect("TLT intent exists");
    assert_eq!(spy.symbol, sym("SPY"));
    assert_eq!(spy.side, Side::Buy);
    assert_eq!(tlt.symbol, sym("TLT"));
    assert_eq!(tlt.side, Side::Sell);

    let limit = Decimal::from(50000);
    for row in [&spy, &tlt] {
        assert_eq!(row.state, OrderState::Intent);
        assert_eq!(row.decision_id, Some(decision_id));
        assert_eq!(row.leg, 1);
        assert_eq!(row.trade_date, d(2024, 3, 1));
        assert_eq!(row.broker_order_id, None);
        assert_eq!(row.filled_qty, 0);
        assert!(row.qty > 0);
        assert!(row.reference_price > Decimal::ZERO);
        assert!(Decimal::from(row.qty) * row.reference_price <= limit);
    }
}

// R-303
#[test]
fn writing_the_intents_sends_nothing_to_the_broker() {
    let fx = ready_paper_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert_eq!(posts(&fx), 0);
    assert!(all_orders(&fx)
        .iter()
        .all(|o| o.state == OrderState::Intent));
}

// R-303
#[test]
fn a_second_call_for_the_same_signal_date_is_not_due_and_adds_nothing() {
    let fx = ready_paper_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("first rebalance");
    let before = all_orders(&fx);
    assert_eq!(before.len(), 2);

    let again = rebalance(&fx.ctx(), &wire).expect("second rebalance");
    assert_eq!(again, RebalanceOutcome::NotDue);
    assert_eq!(all_orders(&fx), before);
    assert_eq!(fx.store.decisions(Mode::Paper).expect("decisions").len(), 1);
    assert_eq!(posts(&fx), 0);
}

/// A fresh ready fixture holding only the decision `rebalance` records for 2024-02-29 and,
/// of its orders, those named in `keep`. Returns the fixture, the stored decision id and the
/// orders a complete run writes (with the decision id of the fresh fixture).
fn partial_fixture(keep: &[&str]) -> (LiveFixture, i64, Vec<pt_store::trading::OrderRow>) {
    let source = ready_paper_fixture();
    let wire = clock(&source.http).expect("clock");
    rebalance(&source.ctx(), &wire).expect("source rebalance");
    let decision = source
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("source decision");
    let complete = all_orders(&source);
    assert_eq!(complete.len(), 2);

    let fx = ready_paper_fixture();
    let decision_id = fx
        .store
        .insert_decision(&decision)
        .expect("insert decision");
    let expected: Vec<_> = complete
        .into_iter()
        .map(|mut o| {
            o.decision_id = Some(decision_id);
            o
        })
        .collect();
    for o in expected
        .iter()
        .filter(|o| keep.contains(&o.client_order_id.as_str()))
    {
        fx.store.insert_order(o).expect("insert kept order");
    }
    (fx, decision_id, expected)
}

fn assert_backfilled(keep: &[&str]) {
    let (fx, decision_id, expected) = partial_fixture(keep);
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert_eq!(outcome, RebalanceOutcome::NotDue);

    let rows = all_orders(&fx);
    assert_eq!(rows, expected);
    let ids: Vec<_> = rows.iter().map(|o| o.client_order_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["pt-tsmom-v1-20240229-SPY-1", "pt-tsmom-v1-20240229-TLT-1"]
    );
    assert!(rows
        .iter()
        .all(|o| o.decision_id == Some(decision_id) && o.state == OrderState::Intent));
    assert_eq!(fx.store.decisions(Mode::Paper).expect("decisions").len(), 1);
    assert_eq!(posts(&fx), 0);
}

// R-303
#[test]
fn a_stored_decision_without_orders_gets_its_orders_written_once() {
    assert_backfilled(&[]);
}

// R-303
#[test]
fn a_stored_decision_with_one_of_two_orders_gets_the_missing_one_written_once() {
    assert_backfilled(&["pt-tsmom-v1-20240229-SPY-1"]);
}
