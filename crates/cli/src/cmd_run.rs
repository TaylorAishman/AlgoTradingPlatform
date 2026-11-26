//! The `run` command: build the live context and run one daily cycle.

use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::clock::SystemClock;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;
use pt_gate::gate::Gate;
use pt_live::context::LiveContext;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_live::error::LiveError;
use pt_live::lock::acquire_run_lock;
use pt_ops::alert::Alerter;
use pt_ops::report::{write_status, ReportInputs};
use pt_shadow::{daily, slippage};
use pt_store::db::Store;
use pt_strategy::error::StrategyError;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};

use crate::wiring;

/// The version recorded with every run.
const CODE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Time-series momentum on the calendar in the store at the time of each call.
///
/// The cycle syncs the calendar before it decides, so a fresh store has no
/// calendar when the run starts; reading it per call picks up that sync.
struct StoredTsmom<'a> {
    store: &'a Store,
    params: TsmomParams,
    /// Answers `id` and `params_canonical`, which do not read the calendar.
    template: Tsmom,
}

impl<'a> StoredTsmom<'a> {
    fn new(store: &'a Store, params: TsmomParams, start: NaiveDate) -> anyhow::Result<Self> {
        let placeholder = TradingCalendar::new(vec![start]).context("strategy calendar")?;
        let template = Tsmom::new(params.clone(), Arc::new(placeholder));
        Ok(Self {
            store,
            params,
            template,
        })
    }
}

impl Strategy for StoredTsmom<'_> {
    fn id(&self) -> &str {
        self.template.id()
    }

    fn params_canonical(&self) -> String {
        self.template.params_canonical()
    }

    fn signals(
        &self,
        universe: &[Symbol],
        asof: NaiveDate,
        view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError> {
        let calendar = self
            .store
            .calendar()
            .map_err(|e| StrategyError::Invalid(format!("reading the calendar: {e}")))?;
        Tsmom::new(self.params.clone(), Arc::new(calendar)).signals(universe, asof, view)
    }
}

fn open_store(db_path: &str) -> anyhow::Result<Store> {
    let path = Path::new(db_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    Store::open(path).with_context(|| format!("opening store {}", path.display()))
}

/// The alert webhook: `PT_ALERT_WEBHOOK_URL`, else the owner file's.
fn webhook(owner_url: Option<String>) -> Option<String> {
    match std::env::var("PT_ALERT_WEBHOOK_URL") {
        Ok(url) if !url.is_empty() => Some(url),
        _ => owner_url,
    }
}

/// Run one cycle. Exits 0 when it ends ok, 1 when it fails, 2 when it is
/// halted and 3 when another run holds the lock.
pub fn run() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
    let keys = wiring::load_keys()?;
    let store = open_store(&cfg.run.db_path)?;
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
        webhook(owner.alert_webhook_url.clone()),
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
