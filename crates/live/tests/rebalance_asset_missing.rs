//! R-507: an asset with no stored snapshot is left out of the rebalance with an `asset_missing`
//! warning.

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar};
use pt_data::reference::sync_assets;
use pt_engine::decision::{record_from_json, DecisionRecord};
use pt_gate::owner::Stage;
use pt_live::rebalance::{rebalance, RebalanceOutcome};
use pt_live::reconcile::reconcile;
use pt_store::market::{AssetSnapshot, BarRow};
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

/// A paper fixture; with `store_tlt` false, only SPY's asset snapshot is stored.
fn fixture(store_tlt: bool) -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    seed_bars(&fx);
    if store_tlt {
        sync_assets(&fx.store, &fx.http, &fx.cfg, fx.now()).expect("assets");
    } else {
        fx.store
            .put_asset_snapshot(
                &AssetSnapshot {
                    symbol: sym("SPY"),
                    tradable: true,
                    shortable: true,
                    easy_to_borrow: true,
                },
                fx.now(),
            )
            .expect("spy snapshot");
    }
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

// R-507
#[test]
fn an_asset_with_no_snapshot_is_absent_from_the_universe() {
    let fx = fixture(false);
    let record = decide(&fx);
    assert!(!record.inputs.universe.contains(&sym("TLT")));
    assert_eq!(record.inputs.universe, vec![sym("SPY")]);
}

// R-507
#[test]
fn an_asset_with_no_snapshot_records_one_asset_missing_warning_naming_it() {
    let fx = fixture(false);
    decide(&fx);
    let alerts = fx.store.alerts().expect("alerts");
    let missing: Vec<_> = alerts.iter().filter(|a| a.1 == "asset_missing").collect();
    assert_eq!(missing.len(), 1, "alerts: {alerts:?}");
    assert_eq!(missing[0].0, "warn");
    assert!(missing[0].2.contains("TLT"), "message: {}", missing[0].2);
}

// R-507
#[test]
fn with_every_snapshot_stored_no_asset_missing_alert_exists() {
    let fx = fixture(true);
    decide(&fx);
    assert!(fx
        .store
        .alerts()
        .expect("alerts")
        .iter()
        .all(|a| a.1 != "asset_missing"));
}
