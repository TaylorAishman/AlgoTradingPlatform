//! R-340: a rebalance traded later than the session after its signal date
//! counts as late.

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
use pt_shadow::fidelity::fidelity;
use pt_store::db::Store;
use pt_store::trading::{DecisionRow, Mode};
use pt_strategy::benchmark::LongOnly;
use std::collections::BTreeSet;
use std::sync::Arc;

fn strategy() -> LongOnly {
    let sessions = vec![d(2024, 1, 2), d(2024, 1, 3), d(2024, 1, 4), d(2024, 1, 5)];
    LongOnly::new(Arc::new(TradingCalendar::new(sessions).unwrap()))
}

/// Stores one paper decision for signal date 2024-01-02 with the given trade
/// date, status and bootstrap flag. The strategy id carries the index `n`
/// because the store requires (strategy_id, signal_date, mode) to be unique,
/// and every row here shares the signal date and mode.
fn add_decision(
    store: &Store,
    cfg: &Config,
    strategy: &LongOnly,
    n: usize,
    trade: chrono::NaiveDate,
    status: &str,
    bootstrap: bool,
) {
    let universe: Vec<Symbol> = vec![sym("A"), sym("B")];
    let inputs = DecisionInputs {
        signal_date: d(2024, 1, 2),
        trade_date: trade,
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
        bootstrap,
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
            strategy_id: format!("bh-volscaled-v1-{n}"),
            signal_date: d(2024, 1, 2),
            trade_date: Some(trade),
            mode: Mode::Paper,
            status: status.to_string(),
            inputs_json: record_to_json(&record),
            outputs_json: decision_to_json(&decision),
            created_at: at(2024, 1, 2, 22, 0),
        })
        .unwrap();
}

fn late(rows: &[(chrono::NaiveDate, &str, bool)]) -> usize {
    let store = shadow_store();
    let cfg = config();
    let strategy = strategy();
    for (n, (trade, status, bootstrap)) in rows.iter().enumerate() {
        add_decision(&store, &cfg, &strategy, n, *trade, status, *bootstrap);
    }
    fidelity(&store, &cfg, &strategy).unwrap().late_rebalances
}

// R-340
#[test]
fn a_planned_rebalance_traded_two_sessions_after_its_signal_is_late() {
    assert_eq!(late(&[(d(2024, 1, 4), "planned", false)]), 1);
}

// R-340
#[test]
fn a_bootstrap_rebalance_is_never_late() {
    assert_eq!(late(&[(d(2024, 1, 4), "planned", true)]), 0);
}

// R-340
#[test]
fn a_rebalance_traded_on_the_next_session_is_not_late() {
    assert_eq!(late(&[(d(2024, 1, 3), "planned", false)]), 0);
}

// R-340
#[test]
fn a_skipped_late_decision_adds_one() {
    assert_eq!(late(&[(d(2024, 1, 3), "skipped_late", false)]), 1);
}

// R-340
#[test]
fn late_and_skipped_decisions_are_summed() {
    let n = late(&[
        (d(2024, 1, 4), "planned", false),
        (d(2024, 1, 4), "planned", true),
        (d(2024, 1, 3), "skipped_late", false),
        (d(2024, 1, 3), "planned", false),
    ]);

    assert_eq!(n, 2);
}

// R-340
#[test]
fn the_shadow_fixture_has_no_late_rebalances() {
    assert_eq!(late(&[]), 0);
}
