//! R-338: the fidelity summary counts sessions, rebalances, fills and breaks.

mod common;

use common::*;
use pt_calendar::calendar::TradingCalendar;
use pt_config::config::Config;
use pt_config::hash::config_hash;
use pt_core::money::Money;
use pt_core::types::{Positions, PriceBasis, Symbol};
use pt_data::view::StoreView;
use pt_engine::decision::{
    decide, decision_to_json, params_from_config, record_to_json, DecisionInputs, DecisionRecord,
};
use pt_shadow::daily::update;
use pt_shadow::fidelity::fidelity;
use pt_shadow::slippage::record_slippage;
use pt_store::db::Store;
use pt_store::trading::{DecisionRow, Mode};
use pt_strategy::benchmark::LongOnly;
use std::collections::BTreeSet;
use std::sync::Arc;

fn strategy() -> LongOnly {
    let sessions = vec![d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)];
    LongOnly::new(Arc::new(TradingCalendar::new(sessions).unwrap()))
}

/// Stores one `planned` paper decision (signal 2024-01-02, trade 2024-01-03)
/// that replays: its data is as of before any bar was fetched, so every
/// weight is zero and no order results.
fn add_replaying_decision(store: &Store, cfg: &Config, strategy: &LongOnly) {
    let universe: Vec<Symbol> = vec![sym("A"), sym("B")];
    let inputs = DecisionInputs {
        signal_date: d(2024, 1, 2),
        trade_date: d(2024, 1, 3),
        ref_date: d(2024, 1, 2),
        equity: Money(dec("100000")),
        positions: Positions::new(),
        universe: universe.clone(),
        shortable: universe.iter().cloned().collect::<BTreeSet<_>>(),
    };
    let data_asof = at(2024, 1, 1, 0, 0);
    let record = DecisionRecord {
        inputs: inputs.clone(),
        data_asof,
        feed: "sip".to_string(),
        config_hash: config_hash(cfg),
        code_version: "test".to_string(),
        bootstrap: false,
    };
    let mut symbols = universe;
    symbols.push(sym(&cfg.strategy.risk_free_symbol));
    symbols.sort();
    symbols.dedup();
    let view = StoreView::load(store, &symbols, "sip", PriceBasis::Raw, Some(data_asof)).unwrap();
    let decision = decide(&inputs, strategy, &view, &params_from_config(cfg)).unwrap();
    store
        .insert_decision(&DecisionRow {
            id: 0,
            strategy_id: "bh-volscaled-v1".to_string(),
            signal_date: d(2024, 1, 2),
            trade_date: Some(d(2024, 1, 3)),
            mode: Mode::Paper,
            status: "planned".to_string(),
            inputs_json: record_to_json(&record),
            outputs_json: decision_to_json(&decision),
            created_at: at(2024, 1, 2, 22, 0),
        })
        .unwrap();
}

fn prepared() -> (Store, Config, LongOnly) {
    let store = shadow_store();
    let cfg = config();
    let strategy = strategy();
    update(&store, &cfg).unwrap();
    record_slippage(&store, &cfg).unwrap();
    add_replaying_decision(&store, &cfg, &strategy);
    (store, cfg, strategy)
}

fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() < tol, "{a} != {b}");
}

// R-338
#[test]
fn fixture_counts_sessions_rebalances_fills_and_breaks() {
    let (store, cfg, strategy) = prepared();

    let f = fidelity(&store, &cfg, &strategy).unwrap();

    assert_eq!(f.n_sessions, 2);
    assert_eq!(f.n_rebalances, 1);
    assert_eq!(f.n_fills, 2);
    assert_eq!(f.replay_mismatches, 0);
    assert_eq!(f.reconciliation_breaks, 0);
}

// R-338
#[test]
fn fixture_means_and_maximum_of_slippage_and_differences() {
    let (store, cfg, strategy) = prepared();

    let f = fidelity(&store, &cfg, &strategy).unwrap();

    close(f.mean_slippage_bps, 5.536472, 1e-6);
    close(f.mean_diff_bps, 1.75, 1e-9);
    close(f.max_abs_diff_bps, 3.5, 1e-9);
}

// R-338
#[test]
fn reconciliation_halts_count_as_breaks_cleared_or_not() {
    let (store, cfg, strategy) = prepared();
    store
        .set_halt(
            "reconciliation_position",
            "A differs",
            at(2024, 1, 3, 22, 0),
        )
        .unwrap();
    let foreign = store
        .set_halt("foreign_order", "unknown order", at(2024, 1, 3, 23, 0))
        .unwrap();
    store
        .clear_halt(foreign, "reviewed", at(2024, 1, 4, 9, 0))
        .unwrap();

    let f = fidelity(&store, &cfg, &strategy).unwrap();

    assert_eq!(f.reconciliation_breaks, 2);
}

// R-338
#[test]
fn a_daily_loss_halt_is_not_a_break() {
    let (store, cfg, strategy) = prepared();
    store
        .set_halt("daily_loss", "down 3%", at(2024, 1, 3, 22, 0))
        .unwrap();

    let f = fidelity(&store, &cfg, &strategy).unwrap();

    assert_eq!(f.reconciliation_breaks, 0);
}
