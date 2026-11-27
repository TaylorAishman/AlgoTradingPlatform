//! R-506: a rebalance writes its decision and its orders together, or neither.

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_core::types::{Adjustment, Bar};
use pt_data::reference::sync_assets;
use pt_gate::owner::Stage;
use pt_live::rebalance::{rebalance, RebalanceOutcome};
use pt_live::reconcile::reconcile;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, Mode, OrderRow};
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

const SPY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";
const TLT_ID: &str = "pt-tsmom-v1-20240229-TLT-1";

// R-506
#[test]
fn a_rebalance_links_both_orders_to_its_decision() {
    let fx = ready_paper_fixture();
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    let RebalanceOutcome::Recorded {
        decision_id,
        orders: 2,
    } = outcome
    else {
        panic!("expected Recorded with 2 orders, got {outcome:?}");
    };
    for id in [SPY_ID, TLT_ID] {
        let row = fx.store.order(id).expect("lookup").expect("order exists");
        assert_eq!(row.decision_id, Some(decision_id), "order {id}");
    }
}

// R-506
#[test]
fn a_planned_order_id_already_stored_fails_the_rebalance_and_leaves_nothing_behind() {
    let fx = ready_paper_fixture();
    fx.store
        .insert_order(&OrderRow {
            client_order_id: SPY_ID.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Buy,
            qty: 1,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::from(100),
            trade_date: d(2024, 3, 1),
        })
        .expect("stray order");
    let wire = clock(&fx.http).expect("clock");

    let result = rebalance(&fx.ctx(), &wire);
    assert!(result.is_err(), "expected an error, got {result:?}");

    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
    assert!(fx.store.order(TLT_ID).expect("lookup").is_none());
    let spy = fx
        .store
        .order(SPY_ID)
        .expect("lookup")
        .expect("stray stays");
    assert_eq!(spy.decision_id, None);
    assert_eq!(spy.qty, 1);
}
