//! The monthly rebalance decision (architecture D-02, D-23).
//!
//! The due signal date comes from the stored calendar and the monthly schedule.
//! The decision is computed by the shared decision function over the stored
//! data and recorded with its inputs before any order exists.

use std::collections::BTreeSet;

use chrono::NaiveDate;

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
    if let Some(existing) =
        ctx.store
            .decision_for(ctx.strategy.id(), due.signal_date, Mode::Paper)?
    {
        // A run may have stopped between recording the decision and writing all
        // of its orders; write any that are missing from the stored decision.
        let trade_date = existing.trade_date.ok_or_else(|| {
            LiveError::Invalid(format!("decision {} has no trade date", existing.id))
        })?;
        let orders = stored_orders(&existing.outputs_json)?;
        write_orders(ctx, existing.id, due.signal_date, trade_date, &orders)?;
        return Ok(RebalanceOutcome::NotDue);
    }
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
    write_orders(ctx, decision_id, due.signal_date, trade_date, &orders)?;
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

/// Writes each planned order as an Intent row of the decision. The store has
/// no transaction spanning calls, so each order is its own atomic write; an
/// order whose deterministic client order id is already stored counts as
/// written, so a rerun after a partial write completes the set without
/// duplicating any order.
fn write_orders(
    ctx: &LiveContext,
    decision_id: i64,
    signal_date: NaiveDate,
    trade_date: NaiveDate,
    orders: &[PlannedOrder],
) -> Result<(), LiveError> {
    for o in orders {
        let res = ctx.store.insert_order(&OrderRow {
            client_order_id: client_order_id(ctx.strategy.id(), signal_date, o.symbol.as_str(), 1),
            decision_id: Some(decision_id),
            symbol: o.symbol.clone(),
            side: o.side,
            qty: o.qty,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: o.reference_price,
            trade_date,
        });
        match res {
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
