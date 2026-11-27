//! The monthly rebalance decision (architecture D-02, D-23).
//!
//! The due signal date comes from the stored calendar and the monthly schedule.
//! The decision is computed by the shared decision function over the stored
//! data and recorded with its inputs before any order exists.

use std::collections::BTreeSet;

use chrono::NaiveDate;

use pt_alpaca::account;
use pt_alpaca::reference::WireClock;
use pt_calendar::schedule::monthly_schedule;
use pt_config::hash::config_hash;
use pt_core::money::Money;
use pt_core::order_state::OrderState;
use pt_core::types::{PriceBasis, Side, Symbol};
use pt_core::view::MarketView;
use pt_data::view::StoreView;
use pt_engine::decision::{
    decide, decision_to_json, params_from_config, record_to_json, DecisionInputs, DecisionRecord,
};
use pt_gate::gate::Capability;
use pt_store::error::StoreError;
use pt_store::trading::{Book, DecisionRow, Mode, OrderRow};
use rust_decimal::Decimal;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::ids::client_order_id;
use crate::snapshot::trade_session;

/// What one call of [`rebalance`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalanceOutcome {
    /// No signal date is due.
    NotDue,
    /// A decision was recorded with this many planned orders.
    Recorded { decision_id: i64, orders: usize },
    /// A needed symbol has no stored bar for the reference date; nothing was decided.
    DataNotReady,
    /// The trade date is past the allowed delay; the decision was stored as skipped.
    SkippedLate,
    /// Too few minutes remain before the close; nothing was written (R-307).
    Deferred,
}

/// Decides the rebalance for the latest month-end signal date whose trade
/// date has arrived, and records it as a `planned` decision with its inputs.
/// Below the first-order stage the decision is recorded in mode `DryRun`, its
/// orders in state `DryRun` with `dry-` ids, and equity comes from the broker
/// account. Nothing is sent to the broker.
pub fn rebalance(ctx: &LiveContext, wire_clock: &WireClock) -> Result<RebalanceOutcome, LiveError> {
    let calendar = ctx.store.calendar()?;
    let trade_date = trade_session(wire_clock);
    let Some(due) = monthly_schedule(&calendar, ctx.cfg.data.start, trade_date).pop() else {
        return Ok(RebalanceOutcome::NotDue);
    };
    // Below the first-order stage nothing may be sent: the decision and its
    // orders are only recorded, in their own mode, and only where the stage
    // allows recording a dry run.
    let dry = ctx.gate.allows(Capability::SubmitOrder).is_err();
    if dry {
        ctx.gate.allows(Capability::RecordDryRun)?;
    }
    let mode = if dry { Mode::DryRun } else { Mode::Paper };
    if let Some(existing) = ctx
        .store
        .decision_for(ctx.strategy.id(), due.signal_date, mode)?
    {
        // A skipped decision has no orders to write; its outputs are kept only
        // so it can be replayed.
        if existing.status == "skipped_late" {
            return Ok(RebalanceOutcome::NotDue);
        }
        // A run may have stopped between recording the decision and writing all
        // of its orders; write any that are missing from the stored decision.
        let trade_date = existing.trade_date.ok_or_else(|| {
            LiveError::Invalid(format!("decision {} has no trade date", existing.id))
        })?;
        let orders = stored_orders(&existing.outputs_json)?;
        write_orders(ctx, existing.id, due.signal_date, trade_date, &orders, dry)?;
        return Ok(RebalanceOutcome::NotDue);
    }
    // Too close to the close: write nothing, so a later call can decide (R-307).
    let minutes_left = (wire_clock.next_close - wire_clock.timestamp).num_minutes();
    if minutes_left < i64::from(ctx.cfg.risk.min_minutes_before_close) {
        return Ok(RebalanceOutcome::Deferred);
    }
    // The normal trade date is the session after the signal date; each session
    // after it counts as one session late. Exactly at the limit is on time.
    let normal = calendar.next_session(due.signal_date).ok_or_else(|| {
        LiveError::Invalid(format!("no session after signal date {}", due.signal_date))
    })?;
    let late = calendar
        .sessions_between(normal, trade_date)
        .len()
        .saturating_sub(1);
    // The first rebalance of a mode is allowed however late it is (R-306).
    let bootstrap = ctx.store.decisions(mode)?.is_empty();
    // A late rebalance is still decided below, so its skipped row carries a
    // full record and outputs that replay like any other decision.
    let skip_late = !bootstrap && late > ctx.cfg.risk.max_rebalance_delay_sessions as usize;
    let ref_date = calendar
        .prev_session(trade_date)
        .ok_or_else(|| LiveError::Invalid(format!("no session before trade date {trade_date}")))?;

    let equity = if dry {
        // No paper ledger or snapshot exists yet; the broker account is the source.
        account::account(ctx.http)?.equity
    } else {
        ctx.store
            .equity_series(Book::Paper)?
            .into_iter()
            .find(|s| s.date == ref_date)
            .ok_or_else(|| LiveError::Invalid(format!("no paper equity snapshot for {ref_date}")))?
            .equity
    };
    let positions = ctx.store.positions(Book::Paper)?;

    let mut universe = Vec::new();
    let mut shortable = BTreeSet::new();
    let mut untradable = Vec::new();
    for inst in &ctx.cfg.universe {
        let symbol = symbol(&inst.symbol)?;
        let Some(asset) = ctx.store.asset_asof(&symbol, None)? else {
            continue;
        };
        if !asset.tradable {
            untradable.push(symbol);
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
    let missing: Vec<&str> = symbols
        .iter()
        .filter(|s| view.exec_bar(s, ref_date).is_none())
        .map(Symbol::as_str)
        .collect();
    if !missing.is_empty() {
        ctx.store.record_alert(
            "warn",
            "data_not_ready",
            &format!("no bar for {} on {ref_date}", missing.join(", ")),
            false,
            ctx.clock.now_utc(),
        )?;
        return Ok(RebalanceOutcome::DataNotReady);
    }
    let decision = decide(&inputs, ctx.strategy, &view, &params_from_config(ctx.cfg))?;

    let now = ctx.clock.now_utc();
    let record = DecisionRecord {
        inputs,
        data_asof: now,
        feed,
        config_hash: config_hash(ctx.cfg),
        code_version: ctx.code_version.to_string(),
        bootstrap,
    };
    if skip_late {
        // The alert is written before the row: if it fails, no row exists and
        // a rerun tries both again. Nothing is written as an order or sent.
        ctx.store.record_alert(
            "warn",
            "late_rebalance_skipped",
            &format!(
                "rebalance for signal date {} is {late} sessions late on {trade_date}; skipped",
                due.signal_date
            ),
            false,
            now,
        )?;
        ctx.store.insert_decision(&DecisionRow {
            id: 0,
            strategy_id: ctx.strategy.id().to_string(),
            signal_date: due.signal_date,
            trade_date: Some(trade_date),
            mode,
            status: "skipped_late".to_string(),
            inputs_json: record_to_json(&record),
            outputs_json: decision_to_json(&decision),
            created_at: now,
        })?;
        return Ok(RebalanceOutcome::SkippedLate);
    }
    let orders: Vec<PlannedOrder> = decision
        .orders
        .iter()
        .map(|o| PlannedOrder {
            symbol: o.symbol.clone(),
            side: o.side,
            qty: o.qty,
            reference_price: o.reference_price.value(),
        })
        .collect();
    let rows = order_rows(ctx, due.signal_date, trade_date, &orders, dry);
    let decision_id = ctx.store.insert_decision_with_orders(
        &DecisionRow {
            id: 0,
            strategy_id: ctx.strategy.id().to_string(),
            signal_date: due.signal_date,
            trade_date: Some(trade_date),
            mode,
            status: "planned".to_string(),
            inputs_json: record_to_json(&record),
            outputs_json: decision_to_json(&decision),
            created_at: now,
        },
        &rows,
    )?;
    // After the decision is stored, so a rerun (which returns above) or a
    // data-not-ready retry never repeats an alert for the same decision.
    for symbol in &untradable {
        ctx.store.record_alert(
            "warn",
            "not_tradable",
            &format!("{symbol} is not tradable; left out of the universe"),
            false,
            now,
        )?;
    }
    Ok(RebalanceOutcome::Recorded {
        decision_id,
        orders: decision.orders.len(),
    })
}

/// One planned order as stored in a decision's outputs.
struct PlannedOrder {
    symbol: Symbol,
    side: Side,
    qty: u64,
    reference_price: Decimal,
}

/// The rows for planned orders: `Intent` with the canonical client order id,
/// or when `dry` is set `DryRun` with that id prefixed `dry-`, so a dry-run row
/// never takes the id the paper order will send and adopt by.
fn order_rows(
    ctx: &LiveContext,
    signal_date: NaiveDate,
    trade_date: NaiveDate,
    orders: &[PlannedOrder],
    dry: bool,
) -> Vec<OrderRow> {
    orders
        .iter()
        .map(|o| {
            let id = client_order_id(ctx.strategy.id(), signal_date, o.symbol.as_str(), 1);
            OrderRow {
                client_order_id: if dry { format!("dry-{id}") } else { id },
                decision_id: None,
                symbol: o.symbol.clone(),
                side: o.side,
                qty: o.qty,
                leg: 1,
                state: if dry {
                    OrderState::DryRun
                } else {
                    OrderState::Intent
                },
                broker_order_id: None,
                filled_qty: 0,
                reference_price: o.reference_price,
                trade_date,
            }
        })
        .collect()
}

/// Writes the orders a stored decision is missing, for a run that stopped
/// after recording the decision. A new decision and its orders are written
/// together by `insert_decision_with_orders`; here each order is its own
/// atomic write, and an order whose deterministic client order id is already
/// stored counts as written, so a rerun completes the set without duplicates.
fn write_orders(
    ctx: &LiveContext,
    decision_id: i64,
    signal_date: NaiveDate,
    trade_date: NaiveDate,
    orders: &[PlannedOrder],
    dry: bool,
) -> Result<(), LiveError> {
    for row in order_rows(ctx, signal_date, trade_date, orders, dry) {
        match ctx.store.insert_order(&OrderRow {
            decision_id: Some(decision_id),
            ..row
        }) {
            Ok(()) | Err(StoreError::Duplicate(_)) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// The planned orders of a stored decision's `outputs_json` (its `orders` array).
fn stored_orders(outputs_json: &str) -> Result<Vec<PlannedOrder>, LiveError> {
    let bad = |what: &str| LiveError::Invalid(format!("stored decision orders: {what}"));
    let v: serde_json::Value =
        serde_json::from_str(outputs_json).map_err(|e| bad(&e.to_string()))?;
    let list = v
        .get("orders")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| bad("no orders array"))?;
    list.iter()
        .map(|o| {
            let field = |k: &str| o.get(k).ok_or_else(|| bad(&format!("missing {k}")));
            let sym = field("symbol")?.as_str().ok_or_else(|| bad("symbol"))?;
            let side = match field("side")?.as_str() {
                Some("buy") => Side::Buy,
                Some("sell") => Side::Sell,
                _ => return Err(bad("side")),
            };
            let qty = field("qty")?.as_u64().ok_or_else(|| bad("qty"))?;
            let reference_price = field("reference_price")?
                .as_str()
                .and_then(|p| p.parse::<Decimal>().ok())
                .ok_or_else(|| bad("reference_price"))?;
            Ok(PlannedOrder {
                symbol: symbol(sym)?,
                side,
                qty,
                reference_price,
            })
        })
        .collect()
}

fn symbol(s: &str) -> Result<Symbol, LiveError> {
    Symbol::new(s).map_err(|e| LiveError::Invalid(format!("symbol \"{s}\": {e}")))
}
