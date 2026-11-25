//! The monthly rebalance decision (architecture D-02, D-23).
//!
//! The due signal date comes from the stored calendar and the monthly schedule.
//! The decision is computed by the shared decision function over the stored
//! data and recorded with its inputs before any order exists.

use std::collections::BTreeSet;

use pt_alpaca::reference::WireClock;
use pt_calendar::schedule::monthly_schedule;
use pt_config::hash::config_hash;
use pt_core::money::Money;
use pt_core::types::{PriceBasis, Symbol};
use pt_data::view::StoreView;
use pt_engine::decision::{
    decide, decision_to_json, params_from_config, record_to_json, DecisionInputs, DecisionRecord,
};
use pt_store::trading::{Book, DecisionRow, Mode};

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::snapshot::trade_session;

/// What one call of [`rebalance`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalanceOutcome {
    /// No signal date is due.
    NotDue,
    /// A decision was recorded with this many planned orders.
    Recorded { decision_id: i64, orders: usize },
}

/// Decides the rebalance for the latest month-end signal date whose trade
/// date has arrived, and records it as a `planned` paper decision with its
/// inputs. Nothing is sent to the broker.
pub fn rebalance(ctx: &LiveContext, wire_clock: &WireClock) -> Result<RebalanceOutcome, LiveError> {
    let calendar = ctx.store.calendar()?;
    let trade_date = trade_session(wire_clock);
    let Some(due) = monthly_schedule(&calendar, ctx.cfg.data.start, trade_date).pop() else {
        return Ok(RebalanceOutcome::NotDue);
    };
    let ref_date = calendar
        .prev_session(trade_date)
        .ok_or_else(|| LiveError::Invalid(format!("no session before trade date {trade_date}")))?;

    let equity = ctx
        .store
        .equity_series(Book::Paper)?
        .into_iter()
        .find(|s| s.date == ref_date)
        .ok_or_else(|| LiveError::Invalid(format!("no paper equity snapshot for {ref_date}")))?
        .equity;
    let positions = ctx.store.positions(Book::Paper)?;

    let mut universe = Vec::new();
    let mut shortable = BTreeSet::new();
    for inst in &ctx.cfg.universe {
        let symbol = symbol(&inst.symbol)?;
        let Some(asset) = ctx.store.asset_asof(&symbol, None)? else {
            continue;
        };
        if !asset.tradable {
            continue;
        }
        if inst.shortable && asset.shortable && asset.easy_to_borrow {
            shortable.insert(symbol.clone());
        }
        universe.push(symbol);
    }

    let inputs = DecisionInputs {
        signal_date: due.signal_date,
        trade_date,
        ref_date,
        equity: Money(equity),
        positions,
        universe,
        shortable,
    };

    // The same symbols the replay loads, so the decision can be reproduced.
    let mut symbols: BTreeSet<Symbol> = inputs.universe.iter().cloned().collect();
    symbols.extend(inputs.positions.keys().cloned());
    symbols.insert(symbol(&ctx.cfg.strategy.risk_free_symbol)?);
    let symbols: Vec<Symbol> = symbols.into_iter().collect();
    let feed = ctx.cfg.data.feed.clone();
    let view = StoreView::load(ctx.store, &symbols, &feed, PriceBasis::Raw, None)?;
    let decision = decide(&inputs, ctx.strategy, &view, &params_from_config(ctx.cfg))?;

    let now = ctx.clock.now_utc();
    let record = DecisionRecord {
        inputs,
        data_asof: now,
        feed,
        config_hash: config_hash(ctx.cfg),
        code_version: ctx.code_version.to_string(),
        bootstrap: false,
    };
    let decision_id = ctx.store.insert_decision(&DecisionRow {
        id: 0,
        strategy_id: ctx.strategy.id().to_string(),
        signal_date: due.signal_date,
        trade_date: Some(trade_date),
        mode: Mode::Paper,
        status: "planned".to_string(),
        inputs_json: record_to_json(&record),
        outputs_json: decision_to_json(&decision),
        created_at: now,
    })?;
    Ok(RebalanceOutcome::Recorded {
        decision_id,
        orders: decision.orders.len(),
    })
}

fn symbol(s: &str) -> Result<Symbol, LiveError> {
    Symbol::new(s).map_err(|e| LiveError::Invalid(format!("symbol \"{s}\": {e}")))
}
