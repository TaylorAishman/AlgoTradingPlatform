//! R-305: a rebalance later than `max_rebalance_delay_sessions` is skipped and recorded as
//! skipped; one exactly at the limit is still recorded.

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
use pt_store::trading::{Book, DecisionRow, EquitySnapshot, Mode};
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

/// A paper fixture with `bar_count` sessions of bars, paper snapshots on `snapshot_dates`
/// (month, day), and an earlier planned paper decision for 2024-01-31 when `earlier` is set.
fn fixture(bar_count: usize, snapshot_dates: &[(u32, u32)], earlier: bool) -> LiveFixture {
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
    if earlier {
        fx.store
            .insert_decision(&DecisionRow {
                id: 0,
                strategy_id: "tsmom-v1".to_string(),
                signal_date: d(2024, 1, 31),
                trade_date: Some(d(2024, 2, 1)),
                mode: Mode::Paper,
                status: "planned".to_string(),
                inputs_json: "{}".to_string(),
                outputs_json: "{\"orders\":[]}".to_string(),
                created_at: fx.now(),
            })
            .expect("earlier decision");
    }
    fx.cfg.risk.max_rebalance_delay_sessions = 2;
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
    fx.store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::DryRun,
        ])
        .expect("orders")
}

// Sessions after the normal trade date 2024-03-01: 03-04, 03-05, 03-06, 03-07.
const THROUGH_MAR_06: [(u32, u32); 5] = [(2, 29), (3, 1), (3, 4), (3, 5), (3, 6)];
const THROUGH_MAR_04: [(u32, u32); 3] = [(2, 29), (3, 1), (3, 4)];

// R-305
#[test]
fn a_rebalance_four_sessions_late_with_a_limit_of_two_is_skipped_late() {
    let fx = fixture(45, &THROUGH_MAR_06, true);
    let outcome = rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    assert_eq!(outcome, RebalanceOutcome::SkippedLate);
}

// R-305
#[test]
fn a_skipped_late_rebalance_stores_a_skipped_late_decision_and_no_order() {
    let fx = fixture(45, &THROUGH_MAR_06, true);
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("a decision row for 2024-02-29");
    assert_eq!(row.status, "skipped_late");
    assert!(all_orders(&fx).is_empty());
    assert_eq!(posts(&fx), 0);
}

// R-305
#[test]
fn a_skipped_late_rebalance_records_a_late_rebalance_skipped_alert() {
    let fx = fixture(45, &THROUGH_MAR_06, true);
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    let codes: Vec<String> = fx
        .store
        .alerts()
        .expect("alerts")
        .into_iter()
        .map(|a| a.1)
        .collect();
    assert_eq!(
        codes
            .iter()
            .filter(|c| *c == "late_rebalance_skipped")
            .count(),
        1,
        "alerts: {codes:?}"
    );
}

// R-305
#[test]
fn a_rebalance_exactly_two_sessions_late_is_recorded_with_that_trade_date() {
    let fx = fixture(43, &THROUGH_MAR_04, true);
    let outcome = rebalance(&fx.ctx(), &pre_open("2024-03-05")).expect("rebalance");
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
    assert_eq!(row.trade_date, Some(d(2024, 3, 5)));
    assert!(!all_orders(&fx).is_empty());
    assert!(fx
        .store
        .alerts()
        .expect("alerts")
        .iter()
        .all(|a| a.1 != "late_rebalance_skipped"));
}

// R-305, R-340: the skipped row replays like any decision, so it carries a full record.
#[test]
fn a_skipped_late_decision_stores_a_replayable_record() {
    let fx = fixture(45, &THROUGH_MAR_06, true);
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("a decision row for 2024-02-29");
    let record = record_from_json(&row.inputs_json).expect("a replayable record");
    assert_eq!(record.inputs.signal_date, d(2024, 2, 29));
    assert_eq!(record.inputs.trade_date, d(2024, 3, 7));
    assert_eq!(record.inputs.ref_date, d(2024, 3, 6));
    assert!(!record.bootstrap);
    let outputs: serde_json::Value = serde_json::from_str(&row.outputs_json).expect("outputs json");
    assert!(outputs.get("orders").is_some_and(|o| o.is_array()));
    assert!(all_orders(&fx).is_empty());
    assert_eq!(posts(&fx), 0);
}

// R-305
#[test]
fn a_rerun_after_a_late_skip_is_not_due_and_writes_nothing_more() {
    let fx = fixture(45, &THROUGH_MAR_06, true);
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance");
    let again = rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rerun");
    assert_eq!(again, RebalanceOutcome::NotDue);
    let rows = fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .into_iter()
        .filter(|r| r.signal_date == d(2024, 2, 29))
        .count();
    assert_eq!(rows, 1);
    let skips = fx
        .store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == "late_rebalance_skipped")
        .count();
    assert_eq!(skips, 1);
    assert!(all_orders(&fx).is_empty());
    assert_eq!(posts(&fx), 0);
}
