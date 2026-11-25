//! R-304: with no TLT bar stored for the signal date, `rebalance` decides nothing. It returns
//! `DataNotReady`, records a warning alert `data_not_ready`, and writes no decision and no order.

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar};
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

fn data_not_ready_alerts(fx: &LiveFixture) -> Vec<(String, String, String, bool)> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == "data_not_ready")
        .collect()
}

fn missing_tlt_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    seed_bars(&fx, Some("TLT"));
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

// R-304
#[test]
fn a_missing_tlt_bar_makes_the_rebalance_data_not_ready() {
    let fx = missing_tlt_fixture();
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert_eq!(outcome, RebalanceOutcome::DataNotReady);
}

// R-304
#[test]
fn a_missing_tlt_bar_records_one_data_not_ready_warning() {
    let fx = missing_tlt_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");
    let alerts = data_not_ready_alerts(&fx);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, "warn");
}

// R-304
#[test]
fn a_missing_tlt_bar_writes_no_decision_and_no_order() {
    let fx = missing_tlt_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
    assert!(fx
        .store
        .decisions(Mode::DryRun)
        .expect("decisions")
        .is_empty());
    assert!(all_orders(&fx).is_empty());
    for leg in ["SPY", "TLT"] {
        let id = format!("pt-tsmom-v1-20240229-{leg}-1");
        assert!(fx.store.order(&id).expect("lookup").is_none());
    }
}

// R-304
#[test]
fn a_missing_tlt_bar_sends_nothing_to_the_broker() {
    let fx = missing_tlt_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert_eq!(posts(&fx), 0);
}
