//! R-303: a rebalance is decided for the month-end signal date and recorded with its inputs
//! before any order exists. The data is stored directly (the same series the mock serves).

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_config::hash::config_hash;
use pt_core::money::Price;
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

// R-303
#[test]
fn due_signal_date_is_recorded_as_a_planned_paper_decision() {
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

    let decisions = fx.store.decisions(Mode::Paper).expect("decisions");
    assert_eq!(decisions.len(), 1);
    let row = &decisions[0];
    assert_eq!(row.id, decision_id);
    assert_eq!(row.strategy_id, "tsmom-v1");
    assert_eq!(row.signal_date, d(2024, 2, 29));
    assert_eq!(row.trade_date, Some(d(2024, 3, 1)));
    assert_eq!(row.mode, Mode::Paper);
    assert_eq!(row.status, "planned");
    assert!(fx.store.decisions(Mode::DryRun).expect("dry").is_empty());
}

// R-303
#[test]
fn decision_inputs_hold_equity_positions_universe_shortable_feed_and_config_hash() {
    let fx = ready_paper_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");

    let row = fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .remove(0);
    let record = record_from_json(&row.inputs_json).expect("inputs parse as a record");
    let inputs = &record.inputs;
    assert_eq!(inputs.signal_date, d(2024, 2, 29));
    assert_eq!(inputs.trade_date, d(2024, 3, 1));
    assert_eq!(inputs.equity.0, Decimal::from(100000));
    assert!(inputs.positions.values().all(|q| *q == 0));
    assert_eq!(inputs.universe, vec![sym("SPY"), sym("TLT")]);
    assert!(inputs.shortable.contains(&sym("SPY")));
    assert!(inputs.shortable.contains(&sym("TLT")));
    assert_eq!(inputs.shortable.len(), 2);
    assert_eq!(record.feed, "sip");
    assert_eq!(record.config_hash, config_hash(&fx.cfg));
    assert_eq!(record.code_version, "test-version");
    assert!(!row.outputs_json.is_empty());
}

// R-303
#[test]
fn the_decision_is_stored_without_sending_anything_to_the_broker() {
    let fx = ready_paper_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert_eq!(posts(&fx), 0);
}
