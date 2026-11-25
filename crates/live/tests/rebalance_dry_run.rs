//! R-309: in the dry-run stage a rebalance records what it would do and sends nothing; the
//! paper stage then records its own decision for the same signal date.

mod common;

use chrono::Utc;
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::clock;
use pt_core::money::Price;
use pt_core::order_state::OrderState;
use pt_core::types::{Adjustment, Bar};
use pt_data::reference::sync_assets;
use pt_engine::decision::record_from_json;
use pt_gate::gate::Gate;
use pt_gate::owner::{OwnerFile, Stage};
use pt_live::error::LiveError;
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

/// Dry-run fixture after ingest and sync_assets: no reconcile, no snapshot, empty ledger.
fn dry_run_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::DryRun);
    seed_bars(&fx);
    sync_assets(&fx.store, &fx.http, &fx.cfg, fx.now()).expect("assets");
    fx
}

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST")
        .count()
}

fn orders_in(fx: &LiveFixture, state: OrderState) -> Vec<pt_store::trading::OrderRow> {
    fx.store.orders_in_states(&[state]).expect("orders")
}

// R-309
#[test]
fn dry_run_rebalance_records_a_decision_with_the_account_equity() {
    let fx = dry_run_fixture();
    let wire = clock(&fx.http).expect("clock");
    let outcome = rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { orders: 2, .. }),
        "expected Recorded with 2 orders, got {outcome:?}"
    );

    let decisions = fx.store.decisions(Mode::DryRun).expect("decisions");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].mode, Mode::DryRun);
    assert_eq!(decisions[0].signal_date, d(2024, 2, 29));
    let record = record_from_json(&decisions[0].inputs_json).expect("inputs parse");
    assert_eq!(record.inputs.equity.0, Decimal::from(100000));
    assert!(fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .is_empty());
}

// R-309
#[test]
fn dry_run_orders_are_both_in_state_dry_run() {
    let fx = dry_run_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");

    let dry = orders_in(&fx, OrderState::DryRun);
    assert_eq!(dry.len(), 2);
    let mut symbols: Vec<_> = dry.iter().map(|o| o.symbol.clone()).collect();
    symbols.sort();
    assert_eq!(symbols, vec![sym("SPY"), sym("TLT")]);
    assert!(orders_in(&fx, OrderState::Intent).is_empty());
    assert!(orders_in(&fx, OrderState::Submitted).is_empty());
}

// R-309
#[test]
fn dry_run_rebalance_sends_no_post_to_the_broker() {
    let fx = dry_run_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("rebalance");
    assert_eq!(posts(&fx), 0);
}

// R-309
#[test]
fn paper_stage_records_a_second_decision_for_the_same_signal_date() {
    let mut fx = dry_run_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("dry-run rebalance");

    fx.gate = Gate::new(OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: vec!["tsmom-v1".to_string()],
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        ..OwnerFile::default()
    });
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
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

    let outcome = rebalance(&fx.ctx(), &wire).expect("paper rebalance");
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { .. }),
        "expected Recorded, got {outcome:?}"
    );
    let paper = fx.store.decisions(Mode::Paper).expect("decisions");
    assert_eq!(paper.len(), 1);
    assert_eq!(paper[0].mode, Mode::Paper);
    assert_eq!(paper[0].signal_date, d(2024, 2, 29));
    assert_eq!(
        fx.store.decisions(Mode::DryRun).expect("decisions").len(),
        1
    );
    assert_eq!(posts(&fx), 0);
}

// R-309
#[test]
fn dry_run_rows_carry_the_dry_prefix_and_the_paper_order_keeps_the_canonical_id() {
    let mut fx = dry_run_fixture();
    let wire = clock(&fx.http).expect("clock");
    rebalance(&fx.ctx(), &wire).expect("dry-run rebalance");

    let mut dry_ids: Vec<_> = orders_in(&fx, OrderState::DryRun)
        .into_iter()
        .map(|o| o.client_order_id)
        .collect();
    dry_ids.sort();
    assert_eq!(
        dry_ids,
        vec![
            "dry-pt-tsmom-v1-20240229-SPY-1".to_string(),
            "dry-pt-tsmom-v1-20240229-TLT-1".to_string(),
        ]
    );

    fx.gate = Gate::new(OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: vec!["tsmom-v1".to_string()],
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        ..OwnerFile::default()
    });
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
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
    rebalance(&fx.ctx(), &wire).expect("paper rebalance");

    let spy = fx
        .store
        .order("pt-tsmom-v1-20240229-SPY-1")
        .expect("lookup")
        .expect("paper SPY order keeps the canonical id");
    assert_eq!(spy.state, OrderState::Intent);
    assert_eq!(spy.symbol, sym("SPY"));
    assert_eq!(
        fx.store
            .order("dry-pt-tsmom-v1-20240229-SPY-1")
            .expect("lookup")
            .expect("dry-run SPY row remains")
            .state,
        OrderState::DryRun
    );
}

// R-309
#[test]
fn a_stage_that_may_not_record_a_dry_run_records_nothing() {
    let mut fx = dry_run_fixture();
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::Ingest,
        ..OwnerFile::default()
    });
    let wire = clock(&fx.http).expect("clock");
    let err = rebalance(&fx.ctx(), &wire).expect_err("ingest stage may not record");
    assert!(
        matches!(err, LiveError::Gate(_)),
        "expected a gate error, got {err:?}"
    );
    assert!(fx
        .store
        .decisions(Mode::DryRun)
        .expect("decisions")
        .is_empty());
    assert!(orders_in(&fx, OrderState::DryRun).is_empty());
    assert_eq!(posts(&fx), 0);
}
