//! The daily run cycle (architecture D-02).
//!
//! The stage gate is checked first, so a run in the backtest stage is refused
//! before any request goes to the broker.

use pt_gate::gate::Capability;

use crate::context::LiveContext;
use crate::error::LiveError;

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
pub fn run_cycle(ctx: &LiveContext) -> Result<RunOutcome, LiveError> {
    ctx.gate.allows(Capability::TradingRead)?;
    Ok(RunOutcome {
        run_id: 0,
        status: RunStatus::Ok,
        phases: Vec::new(),
    })
}
