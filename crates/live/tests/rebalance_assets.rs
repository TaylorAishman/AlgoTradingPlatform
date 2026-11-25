//! R-308: the decision's shortable set comes from the stored asset snapshots, and untradable
//! assets are left out of the universe with a `not_tradable` alert.

mod common;

use std::collections::BTreeSet;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar, Symbol};
use pt_data::reference::sync_assets;
use pt_engine::decision::{record_from_json, DecisionRecord};
use pt_gate::owner::Stage;
use pt_live::rebalance::{rebalance, RebalanceOutcome};
use pt_live::reconcile::reconcile;
use pt_store::market::BarRow;
use pt_store::trading::{Book, EquitySnapshot, Mode};
use rust_decimal::Decimal;

fn price(x: f64) -> Price {
    Price::from_f64_shortest(x).expect("price")
}

/// The fixture's closes for the first 41 sessions (through 2024-02-29), for every symbol.
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

/// A paper fixture whose TLT asset has the given (tradable, shortable, easy_to_borrow) flags.
fn fixture(tlt: (bool, bool, bool)) -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_asset("TLT", tlt.0, tlt.1, tlt.2);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    seed_bars(&fx);
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

/// Runs the rebalance and returns the stored record of the decision.
fn decide(fx: &LiveFixture) -> DecisionRecord {
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { .. }),
        "expected Recorded, got {outcome:?}"
    );
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("decision row");
    record_from_json(&row.inputs_json).expect("inputs parse as a record")
}

fn symbols(names: &[&str]) -> BTreeSet<Symbol> {
    names.iter().map(|n| sym(n)).collect()
}

fn tlt_orders(fx: &LiveFixture) -> usize {
    fx.store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::DryRun,
        ])
        .expect("orders")
        .iter()
        .filter(|o| o.symbol == sym("TLT"))
        .count()
}

// R-308
#[test]
fn a_not_shortable_snapshot_leaves_only_spy_in_the_shortable_set() {
    let fx = fixture((true, false, true));
    let record = decide(&fx);
    assert_eq!(record.inputs.shortable, symbols(&["SPY"]));
    assert_eq!(record.inputs.universe, vec![sym("SPY"), sym("TLT")]);
}

// R-308
#[test]
fn a_not_shortable_snapshot_plans_no_tlt_order() {
    let fx = fixture((true, false, true));
    decide(&fx);
    assert_eq!(tlt_orders(&fx), 0);
}

// R-308
#[test]
fn a_hard_to_borrow_snapshot_leaves_only_spy_in_the_shortable_set() {
    let fx = fixture((true, true, false));
    let record = decide(&fx);
    assert_eq!(record.inputs.shortable, symbols(&["SPY"]));
}

// R-308
#[test]
fn a_shortable_easy_to_borrow_snapshot_keeps_both_in_the_shortable_set() {
    let fx = fixture((true, true, true));
    let record = decide(&fx);
    assert_eq!(record.inputs.shortable, symbols(&["SPY", "TLT"]));
}

// R-308
#[test]
fn an_untradable_asset_is_absent_from_the_universe() {
    let fx = fixture((false, true, true));
    let record = decide(&fx);
    assert_eq!(record.inputs.universe, vec![sym("SPY")]);
    assert_eq!(record.inputs.shortable, symbols(&["SPY"]));
    assert_eq!(tlt_orders(&fx), 0);
}

// R-308
#[test]
fn an_untradable_asset_records_a_not_tradable_alert() {
    let fx = fixture((false, true, true));
    decide(&fx);
    let alerts = fx.store.alerts().expect("alerts");
    assert_eq!(
        alerts.iter().filter(|a| a.1 == "not_tradable").count(),
        1,
        "alerts: {alerts:?}"
    );
}

// R-308
#[test]
fn tradable_assets_record_no_not_tradable_alert() {
    let fx = fixture((true, true, true));
    decide(&fx);
    assert!(fx
        .store
        .alerts()
        .expect("alerts")
        .iter()
        .all(|a| a.1 != "not_tradable"));
}
