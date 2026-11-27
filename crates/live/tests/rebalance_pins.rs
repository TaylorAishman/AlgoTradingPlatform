//! R-509, R-510: a decision's `data_asof` is the clock's instant, and a dry-run order id never
//! looks like a real one.

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar};
use pt_data::reference::sync_assets;
use pt_engine::decision::record_from_json;
use pt_gate::owner::Stage;
use pt_live::rebalance::{rebalance, RebalanceOutcome};
use pt_live::reconcile::reconcile;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, Mode};
use rust_decimal::Decimal;

fn price(x: f64) -> Price {
    Price::from_f64_shortest(x).expect("price")
}

/// The fixture's closes for the first 41 sessions (through 2024-02-29).
fn seed_bars(fx: &LiveFixture) {
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

/// A fixture ready to rebalance in `stage`; paper also needs a reconcile and an equity snapshot.
fn fixture(stage: Stage) -> LiveFixture {
    let fx = LiveFixture::new(stage);
    if stage == Stage::Paper {
        assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    }
    seed_bars(&fx);
    sync_assets(&fx.store, &fx.http, &fx.cfg, fx.now()).expect("assets");
    if stage == Stage::Paper {
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
    }
    fx
}

fn run(fx: &LiveFixture) {
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { .. }),
        "expected Recorded, got {outcome:?}"
    );
}

fn order_ids(fx: &LiveFixture) -> Vec<String> {
    let states = [
        OrderState::Intent,
        OrderState::DryRun,
        OrderState::Submitted,
    ];
    fx.store
        .orders_in_states(&states)
        .expect("orders")
        .into_iter()
        .map(|o| o.client_order_id)
        .collect()
}

// R-509
#[test]
fn the_stored_decision_records_the_clock_instant_as_data_asof() {
    let fx = fixture(Stage::Paper);
    run(&fx);
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("decision row");
    let record = record_from_json(&row.inputs_json).expect("record");
    assert_eq!(record.data_asof, fx.now());
}

// R-510
#[test]
fn every_dry_run_order_id_starts_with_dry() {
    let fx = fixture(Stage::DryRun);
    run(&fx);
    let ids = order_ids(&fx);
    assert!(!ids.is_empty());
    assert!(ids.iter().all(|id| id.starts_with("dry-")), "ids: {ids:?}");
}

// R-510
#[test]
fn no_paper_order_id_starts_with_dry() {
    let fx = fixture(Stage::Paper);
    run(&fx);
    let ids = order_ids(&fx);
    assert!(!ids.is_empty());
    assert!(ids.iter().all(|id| !id.starts_with("dry-")), "ids: {ids:?}");
}
