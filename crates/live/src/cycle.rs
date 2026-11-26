//! The daily run cycle (architecture D-02).
//!
//! The stage gate is checked first, so a run in the backtest stage is refused
//! before any request goes to the broker.

use pt_alpaca::reference::{clock, WireClock};
use pt_config::hash::config_hash;
use pt_data::ingest::ingest;
use pt_data::reference::{sync_assets, sync_calendar};
use pt_gate::gate::Capability;
use pt_gate::owner::Stage;

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::rebalance::rebalance;
use crate::snapshot::{expected_last_session, trade_session};

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
    let status = RunStatus::Ok;
    let wire_clock = read_only_phases(ctx, &mut phases)?;

    // Only the dry-run stage records a decision without sending it; the stages
    // that submit orders add their own phase.
    let dry_run_only = ctx.gate.allows(Capability::RecordDryRun).is_ok()
        && ctx.gate.allows(Capability::SubmitOrder).is_err();
    if dry_run_only {
        rebalance(ctx, &wire_clock)?;
        phases.push("rebalance".to_string());
    }

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
