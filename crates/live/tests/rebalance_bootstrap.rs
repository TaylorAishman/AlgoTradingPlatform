//! R-306: the first paper rebalance is allowed however late it is, and its stored record is
//! marked as a bootstrap.

mod common;

use chrono::{DateTime, Utc};
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::WireClock;
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

/// The fixture's closes for the first `count` sessions of 2024, for every symbol.
fn seed_bars(fx: &LiveFixture, count: usize) {
    let sessions = sessions_2024();
    let mut rows = Vec::new();
    let mut prev: [Option<f64>; 3] = [None; 3];
    for (i, date) in sessions.iter().take(count).enumerate() {
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

fn pre_open(date: &str) -> WireClock {
    let at = |t: &str| DateTime::parse_from_rfc3339(&format!("{date}T{t}-05:00")).expect("time");
    WireClock {
        timestamp: at("08:00:00"),
        is_open: false,
        next_open: at("09:30:00"),
        next_close: at("16:00:00"),
    }
}

/// A paper fixture with `bar_count` sessions of bars and paper snapshots on `snapshot_dates`
/// (month, day). There is no earlier paper decision.
fn fixture(bar_count: usize, snapshot_dates: &[(u32, u32)]) -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    seed_bars(&fx, bar_count);
    sync_assets(&fx.store, &fx.http, &fx.cfg, fx.now()).expect("assets");
    for (m, day) in snapshot_dates {
        fx.store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date: d(2024, *m, *day),
                cash: Decimal::from(100000),
                positions_value: Decimal::ZERO,
                equity: Decimal::from(100000),
                recorded_at: fx.now(),
            })
            .expect("snapshot");
    }
    fx.cfg.risk.max_rebalance_delay_sessions = 2;
    fx
}

fn all_orders(fx: &LiveFixture) -> Vec<pt_store::trading::OrderRow> {
    fx.store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::DryRun,
        ])
        .expect("orders")
}

const THROUGH_MAR_06: [(u32, u32); 5] = [(2, 29), (3, 1), (3, 4), (3, 5), (3, 6)];

// R-306
#[test]
fn the_first_rebalance_four_sessions_late_is_recorded_with_the_current_trade_date() {
    let fx = fixture(45, &THROUGH_MAR_06);
    let outcome = rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    let RebalanceOutcome::Recorded { decision_id, .. } = outcome else {
        panic!("expected Recorded, got {outcome:?}");
    };
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("decision row");
    assert_eq!(row.id, decision_id);
    assert_eq!(row.status, "planned");
    assert_eq!(row.trade_date, Some(d(2024, 3, 7)));
}

// R-306
#[test]
fn the_first_rebalance_record_has_bootstrap_set() {
    let fx = fixture(45, &THROUGH_MAR_06);
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("decision row");
    let record = record_from_json(&row.inputs_json).expect("inputs parse as a record");
    assert!(record.bootstrap);
}

// R-306
#[test]
fn the_exempt_first_rebalance_plans_orders_and_records_no_late_alert() {
    let fx = fixture(45, &THROUGH_MAR_06);
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    assert!(!all_orders(&fx).is_empty());
    assert!(fx
        .store
        .alerts()
        .expect("alerts")
        .iter()
        .all(|a| a.1 != "late_rebalance_skipped"));
}
