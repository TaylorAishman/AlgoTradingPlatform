//! R-517: only planned decisions count as rebalances.

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

fn counts(rows: &[(&str, chrono::NaiveDate)]) -> (usize, usize) {
    let store = shadow_store();
    let cfg = config();
    let strategy = strategy();
    for (n, (status, trade)) in rows.iter().enumerate() {
        add_decision(&store, &cfg, &strategy, n, *trade, status, false);
    }
    let f = fidelity(&store, &cfg, &strategy).unwrap();
    (f.n_rebalances, f.late_rebalances)
}

// R-517
#[test]
fn a_planned_and_a_skipped_late_decision_count_one_rebalance_and_one_late() {
    let got = counts(&[("planned", d(2024, 1, 3)), ("skipped_late", d(2024, 1, 3))]);

    assert_eq!(got, (1, 1));
}

// R-517
#[test]
fn a_skipped_late_decision_alone_is_not_a_rebalance() {
    assert_eq!(counts(&[("skipped_late", d(2024, 1, 3))]), (0, 1));
}

// R-517
#[test]
fn a_planned_decision_alone_is_one_rebalance() {
    assert_eq!(counts(&[("planned", d(2024, 1, 3))]), (1, 0));
}
