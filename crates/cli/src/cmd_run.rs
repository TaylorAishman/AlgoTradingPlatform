//! The `run` command: build the live context and run one daily cycle.

use std::path::Path;

use anyhow::Context;
use pt_core::clock::SystemClock;
use pt_core::types::Symbol;
use pt_gate::gate::Gate;
use pt_live::context::LiveContext;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_live::error::LiveError;
use pt_live::lock::acquire_run_lock;
use pt_ops::alert::Alerter;
use pt_ops::report::{write_status, ReportInputs};
use pt_shadow::{daily, slippage};
use pt_strategy::tsmom::TsmomParams;

use crate::strategies::StoredTsmom;
use crate::wiring;

/// The version recorded with every run.
const CODE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run one cycle. Exits 0 when it ends ok, 1 when it fails, 2 when it is
/// halted and 3 when another run holds the lock.
pub fn run() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
    let keys = wiring::load_keys()?;
    let store = wiring::open_store(&cfg.run.db_path)?;
    let _lock = match acquire_run_lock(Path::new(&cfg.run.db_path)) {
        Ok(lock) => lock,
        Err(LiveError::AlreadyRunning) => {
            wiring::log(
                "error",
                &format!("another run holds the lock on {}", cfg.run.db_path),
            );
            return Ok(3);
        }
        Err(e) => return Err(e.into()),
    };
    let http = wiring::build_client()?;

    let alerter = Alerter::new(
        wiring::alert_webhook(owner.alert_webhook_url.clone()),
        vec![
            keys.key_id.expose().to_string(),
            keys.secret_key.expose().to_string(),
        ],
    );
    let stage = wiring::stage_name(owner.stage);
    let signoff = owner.validation_signoff.clone();
    let gate = Gate::new(owner);
    let params = TsmomParams {
        lookback_months: cfg.strategy.lookback_months.clone(),
        risk_free_symbol: Symbol::new(&cfg.strategy.risk_free_symbol)
            .context("strategy.risk_free_symbol")?,
    };
    let strategy = StoredTsmom::new(&store, params, cfg.data.start)?;
    let clock = SystemClock;

    let ctx = LiveContext {
        store: &store,
        cfg: &cfg,
        gate: &gate,
        http: &http,
        clock: &clock,
        alerter: &alerter,
        strategy: &strategy,
        code_version: CODE_VERSION,
    };
    let outcome = run_cycle(&ctx)?;
    // Still holding the lock: compare with the shadow, then write the report.
    // A failure here is logged and does not change the cycle's exit code.
    if let Err(e) = daily::update(&store, &cfg) {
        wiring::log("error", &format!("shadow comparison failed: {e}"));
    }
    if let Err(e) = slippage::record_slippage(&store, &cfg) {
        wiring::log("error", &format!("recording slippage failed: {e}"));
    }
    let report = ReportInputs {
        store: &store,
        cfg: &cfg,
        stage,
        fidelity: &[],
        validation_signoff: signoff.as_deref(),
        now: chrono::Utc::now(),
    };
    if let Err(e) = write_status(&report) {
        wiring::log("error", &format!("writing the report failed: {e}"));
    }
    let status = match outcome.status {
        RunStatus::Ok => "ok",
        RunStatus::Halted => "halted",
        RunStatus::DataNotReady => "data_not_ready",
        RunStatus::Failed => "failed",
    };
    println!(
        "run {} {status}: {}",
        outcome.run_id,
        outcome.phases.join(",")
    );
    match outcome.status {
        RunStatus::Failed => Ok(1),
        RunStatus::Halted => Ok(2),
        _ => Ok(0),
    }
}
