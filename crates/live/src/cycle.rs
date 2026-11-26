//! The daily run cycle (architecture D-02).
//!
//! The stage gate is checked first, so a run in the backtest stage is refused
//! before any request goes to the broker.

use pt_alpaca::reference::{clock, WireClock};
use pt_config::hash::config_hash;
use pt_core::order_state::OrderState;
use pt_data::ingest::ingest;
use pt_data::reference::{sync_assets, sync_calendar};
use pt_gate::gate::{AuthContext, Capability};
use pt_gate::owner::Stage;
use pt_ops::alert::Level;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::first_order::first_order;
use crate::rebalance::{rebalance, RebalanceOutcome};
use crate::reconcile::reconcile;
use crate::settle::settle;
use crate::snapshot::{check_halts, expected_last_session, snapshot_equity, trade_session};
use crate::submit::submit_planned;

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Ok,
    Halted,
    DataNotReady,
    Failed,
}

/// The result of one run: its identifier, final status and the phases it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub run_id: i64,
    pub status: RunStatus,
    pub phases: Vec<String>,
}

/// Run one daily cycle. Refuses the backtest stage before any broker request.
///
/// The run is logged from its start. The read-only phases run in every stage
/// that gets past the gate; the phases that write are added after them by the
/// stages that allow them.
pub fn run_cycle(ctx: &LiveContext) -> Result<RunOutcome, LiveError> {
    ctx.gate.allows(Capability::TradingRead)?;
    let stage = stage_name(ctx.gate.stage());
    let run_id = ctx.store.start_run(
        stage,
        &config_hash(ctx.cfg),
        ctx.code_version,
        ctx.clock.now_utc(),
    )?;

    let mut phases: Vec<String> = Vec::new();
    let status = match run_phases(ctx, &mut phases) {
        Ok(status) => status,
        Err(err) => {
            // A broker or data failure after the run began ends it as failed,
            // with an alert, rather than escaping as an error (R-316).
            ctx.alerter.alert(
                ctx.store,
                Level::Critical,
                failure_code(&err),
                &format!("run {run_id} failed: {err}"),
                None,
                ctx.clock.now_utc(),
            );
            RunStatus::Failed
        }
    };

    ctx.store.finish_run(
        run_id,
        status_name(status),
        &phases.join(","),
        ctx.clock.now_utc(),
    )?;
    Ok(RunOutcome {
        run_id,
        status,
        phases,
    })
}

/// Every phase of a run after it has started, by stage.
fn run_phases(ctx: &LiveContext, phases: &mut Vec<String>) -> Result<RunStatus, LiveError> {
    let mut status = RunStatus::Ok;
    let wire_clock = read_only_phases(ctx, phases)?;

    // Only the dry-run stage records a decision without sending it; the
    // first-order stage settles, reconciles and sends the one approved order;
    // the paper stage settles, reconciles, decides and submits.
    let dry_run_only = ctx.gate.allows(Capability::RecordDryRun).is_ok()
        && ctx.gate.allows(Capability::SubmitOrder).is_err();
    if dry_run_only {
        rebalance(ctx, &wire_clock)?;
        phases.push("rebalance".to_string());
    } else if ctx.gate.stage() == Stage::FirstOrder {
        status = first_order_phases(ctx, &wire_clock, phases)?;
    } else if ctx.gate.stage() == Stage::Paper {
        status = paper_phases(ctx, &wire_clock, phases)?;
    }
    Ok(status)
}

/// The alert code for a failure: broker, feed, or a general run failure.
fn failure_code(err: &LiveError) -> &'static str {
    match err {
        LiveError::Alpaca(_) => "broker_error",
        LiveError::Data(_) => "feed_error",
        _ => "run_failed",
    }
}

/// The phases that only read: the calendar, bars through the last session that
/// must be complete, and the asset flags. Each is pushed once it has finished.
fn read_only_phases(ctx: &LiveContext, phases: &mut Vec<String>) -> Result<WireClock, LiveError> {
    let wire_clock = clock(ctx.http)?;

    sync_calendar(
        ctx.store,
        ctx.http,
        ctx.cfg.data.start,
        trade_session(&wire_clock),
    )?;
    phases.push("calendar".to_string());

    let calendar = ctx.store.calendar()?;
    let through = expected_last_session(&wire_clock, &calendar)
        .ok_or_else(|| LiveError::Invalid("no session before the trade session".to_string()))?;
    ingest(ctx.store, ctx.http, ctx.cfg, through, ctx.clock.now_utc())?;
    phases.push("ingest".to_string());

    sync_assets(ctx.store, ctx.http, ctx.cfg, ctx.clock.now_utc())?;
    phases.push("assets".to_string());
    Ok(wire_clock)
}

/// The phases that guard any submission: settle what is open, reconcile with
/// the broker, snapshot the sessions without a snapshot and check the equity
/// halts. Each is pushed once it has finished. Returns `Some(Halted)` when an
/// unclean reconciliation or an active halt must end the run before anything
/// is decided or sent (D-15, D-21).
fn guard_phases(
    ctx: &LiveContext,
    wire_clock: &WireClock,
    phases: &mut Vec<String>,
) -> Result<Option<RunStatus>, LiveError> {
    settle(ctx)?;
    phases.push("settle".to_string());

    let report = reconcile(ctx)?;
    phases.push("reconcile".to_string());
    if !report.clean {
        return Ok(Some(RunStatus::Halted));
    }

    let calendar = ctx.store.calendar()?;
    let through = expected_last_session(wire_clock, &calendar)
        .ok_or_else(|| LiveError::Invalid("no session before the trade session".to_string()))?;
    snapshot_equity(ctx, through)?;
    phases.push("snapshot".to_string());

    check_halts(ctx)?;
    phases.push("halts".to_string());
    if !ctx.store.active_halts()?.is_empty() {
        return Ok(Some(RunStatus::Halted));
    }
    Ok(None)
}

/// The first-order-stage phases, after the read-only ones: the guard phases,
/// then the owner's approved order is planned and sent through the gate's
/// first-order authorisation (R-319). No rebalance decision is written, and
/// the gate refuses any order other than the approved one, or a second one.
fn first_order_phases(
    ctx: &LiveContext,
    wire_clock: &WireClock,
    phases: &mut Vec<String>,
) -> Result<RunStatus, LiveError> {
    if let Some(status) = guard_phases(ctx, wire_clock, phases)? {
        return Ok(status);
    }

    // Read before planning, so the order planned by this run is not counted.
    let already_sent = first_order_sent(ctx)?;
    first_order(ctx, wire_clock)?;
    phases.push("first_order".to_string());

    let auth = AuthContext {
        strategy_id: ctx.strategy.id().to_string(),
        today: trade_session(wire_clock),
        halted: false,
        risk_reducing: false,
        is_first_order: true,
        first_order_already_sent: already_sent,
    };
    submit_planned(ctx, &auth)?;
    phases.push("submit".to_string());
    Ok(RunStatus::Ok)
}

/// Whether a first order has ever reached the broker: any stored first-order
/// id in a state past `Intent` that a POST produced.
fn first_order_sent(ctx: &LiveContext) -> Result<bool, LiveError> {
    let sent = ctx.store.orders_in_states(&[
        OrderState::Submitted,
        OrderState::PartiallyFilled,
        OrderState::Filled,
        OrderState::Canceled,
        OrderState::Expired,
        OrderState::Rejected,
    ])?;
    Ok(sent
        .iter()
        .any(|o| o.client_order_id.starts_with("pt-first-")))
}

/// The paper-stage phases, after the read-only ones: the guard phases, then
/// decide the rebalance and submit the planned orders. Each is pushed once it
/// has finished.
///
/// An unclean reconciliation or any active halt ends the run as `Halted`
/// before anything is decided or sent (D-15, D-21). A rebalance that finds the
/// data not ready ends it as `DataNotReady`; one that is deferred or skipped as
/// late has no orders to send. Otherwise submission sends every stored
/// `Intent`, including any a stopped run left behind (D-20).
fn paper_phases(
    ctx: &LiveContext,
    wire_clock: &WireClock,
    phases: &mut Vec<String>,
) -> Result<RunStatus, LiveError> {
    if let Some(status) = guard_phases(ctx, wire_clock, phases)? {
        return Ok(status);
    }

    let outcome = rebalance(ctx, wire_clock)?;
    phases.push("rebalance".to_string());
    match outcome {
        RebalanceOutcome::Recorded { .. } | RebalanceOutcome::NotDue => {}
        RebalanceOutcome::DataNotReady => return Ok(RunStatus::DataNotReady),
        RebalanceOutcome::Deferred | RebalanceOutcome::SkippedLate => return Ok(RunStatus::Ok),
    }

    let auth = AuthContext {
        strategy_id: ctx.strategy.id().to_string(),
        today: trade_session(wire_clock),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    };
    submit_planned(ctx, &auth)?;
    phases.push("submit".to_string());
    Ok(RunStatus::Ok)
}

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Backtest => "backtest",
        Stage::Ingest => "ingest",
        Stage::DryRun => "dry_run",
        Stage::FirstOrder => "first_order",
        Stage::Paper => "paper",
    }
}

fn status_name(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Ok => "ok",
        RunStatus::Halted => "halted",
        RunStatus::DataNotReady => "data_not_ready",
        RunStatus::Failed => "failed",
    }
}
