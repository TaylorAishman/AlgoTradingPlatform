//! R-307: a rebalance must not start too close to the close. With an open clock 20 minutes
//! before `next_close` and a 30 minute limit it is deferred and writes nothing; with 45
//! minutes left it is recorded for that session.

mod common;

use chrono::{DateTime, Utc};
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::WireClock;
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

/// An open clock on 2024-03-01 whose timestamp is `time` (Eastern); the close is at 16:00.
fn open_at(time: &str) -> WireClock {
    let at =
        |t: &str| DateTime::parse_from_rfc3339(&format!("2024-03-01T{t}-05:00")).expect("time");
    WireClock {
        timestamp: at(time),
        is_open: true,
        next_open: at("09:30:00")
            .checked_add_days(chrono::Days::new(1))
            .expect("next open"),
        next_close: at("16:00:00"),
    }
}

/// A paper fixture with bars through 2024-02-29, the 02-29 paper snapshot, and a 30 minute
/// limit. There is no earlier paper decision.
fn fixture() -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    seed_bars(&fx, 41);
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
    fx.cfg.risk.min_minutes_before_close = 30;
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

// R-307
#[test]
fn a_rebalance_twenty_minutes_before_the_close_is_deferred() {
    let fx = fixture();
    let outcome = rebalance(&fx.ctx(), &open_at("15:40:00")).expect("rebalance");
    assert_eq!(outcome, RebalanceOutcome::Deferred);
}

// R-307
#[test]
fn a_deferred_rebalance_writes_no_decision_order_or_alert() {
    let fx = fixture();
    let alerts_before = fx.store.alerts().expect("alerts").len();
    rebalance(&fx.ctx(), &open_at("15:40:00")).expect("rebalance");
    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
    assert!(all_orders(&fx).is_empty());
    assert_eq!(fx.store.alerts().expect("alerts").len(), alerts_before);
}

// R-307
#[test]
fn a_rebalance_forty_five_minutes_before_the_close_is_recorded_for_that_session() {
    let fx = fixture();
    let outcome = rebalance(&fx.ctx(), &open_at("15:15:00")).expect("rebalance");
    let RebalanceOutcome::Recorded { decision_id, .. } = outcome else {
        panic!("expected Recorded, got {outcome:?}");
    };
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("decision row");
    assert_eq!(row.id, decision_id);
    assert_eq!(row.trade_date, Some(d(2024, 3, 1)));
}
