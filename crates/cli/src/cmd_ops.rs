//! `halt`, `cancel-open`, `flatten`, `report`, `status` and `signoff` commands (stubs).

use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use chrono::Utc;
use pt_calendar::calendar::TradingCalendar;
use pt_core::clock::SystemClock;
use pt_core::types::Symbol;
use pt_gate::gate::Gate;
use pt_live::context::LiveContext;
use pt_live::error::LiveError;
use pt_ops::alert::Alerter;
use pt_ops::report::{last_ok_run_age_hours, write_status, ReportInputs};
use pt_shadow::fidelity::{fidelity, Fidelity};
use pt_shadow::signoff::{evaluate, write_signoff, Verdict};
use pt_store::db::Store;
use pt_store::error::StoreError;
use pt_strategy::tsmom::{Tsmom, TsmomParams};

use crate::cli::{ConfirmArgs, HaltCmd, SignoffCmd};
use crate::wiring;

fn open_store(db_path: &str) -> anyhow::Result<Store> {
    let path = Path::new(db_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    Store::open(path).with_context(|| format!("opening store {}", path.display()))
}

/// Inspect or change halts. No stage is required and no request is made.
///
/// `status` exits 0 with no active halt and 2 with one; `set` and `clear`
/// exit 0, and `clear` of an unknown or already cleared id exits 1.
pub fn halt(action: &HaltCmd) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store(&cfg.run.db_path)?;
    match action {
        HaltCmd::Status => {
            let active = store.active_halts()?;
            if active.is_empty() {
                println!("no active halts");
                return Ok(0);
            }
            for h in &active {
                println!(
                    "halt {} reason={} set_at={} detail={}",
                    h.id,
                    h.reason,
                    h.set_at.to_rfc3339(),
                    h.detail
                );
            }
            Ok(2)
        }
        HaltCmd::Set { reason } => {
            let id = store.set_halt("manual", reason, Utc::now())?;
            wiring::log("info", &format!("halt {id} set: {reason}"));
            println!("halt {id} set");
            Ok(0)
        }
        HaltCmd::Clear { id, note } => match store.clear_halt(*id, note, Utc::now()) {
            Ok(()) => {
                wiring::log("info", &format!("halt {id} cleared: {note}"));
                println!("halt {id} cleared");
                Ok(0)
            }
            Err(StoreError::NotFound(what)) => {
                wiring::log("error", &format!("no such halt: {what}"));
                Ok(1)
            }
            Err(e) => Err(e.into()),
        },
    }
}

/// Cancel every open order at the broker and print how many cancels were sent.
/// Exits 1 without `--yes`, before any client is built.
pub fn cancel_open(args: &ConfirmArgs) -> anyhow::Result<i32> {
    kill_switch(args, "cancel-open", pt_live::killswitch::cancel_open)
}

/// Close every position and halt, printing how many orders were sent.
/// Exits 1 without `--yes`, before any client is built. Works while halted.
pub fn flatten(args: &ConfirmArgs) -> anyhow::Result<i32> {
    kill_switch(args, "flatten", pt_live::killswitch::flatten)
}

/// Require `--yes`, build the live context as `run` does, call `action` and
/// print its count.
fn kill_switch(
    args: &ConfirmArgs,
    name: &str,
    action: fn(&LiveContext) -> Result<usize, LiveError>,
) -> anyhow::Result<i32> {
    if !args.yes {
        wiring::log("error", &format!("`{name}` needs --yes to confirm"));
        return Ok(1);
    }
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
    let keys = wiring::load_keys()?;
    let store = open_store(&cfg.run.db_path)?;
    let http = wiring::build_client()?;
    let alerter = Alerter::new(
        std::env::var("PT_ALERT_WEBHOOK_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .or_else(|| owner.alert_webhook_url.clone()),
        vec![
            keys.key_id.expose().to_string(),
            keys.secret_key.expose().to_string(),
        ],
    );
    let gate = Gate::new(owner);
    // The kill switch only needs the strategy's id; the calendar is unused.
    let placeholder = TradingCalendar::new(vec![cfg.data.start]).context("strategy calendar")?;
    let strategy = Tsmom::new(
        TsmomParams {
            lookback_months: cfg.strategy.lookback_months.clone(),
            risk_free_symbol: Symbol::new(&cfg.strategy.risk_free_symbol)
                .context("strategy.risk_free_symbol")?,
        },
        Arc::new(placeholder),
    );
    let clock = SystemClock;
    let ctx = LiveContext {
        store: &store,
        cfg: &cfg,
        gate: &gate,
        http: &http,
        clock: &clock,
        alerter: &alerter,
        strategy: &strategy,
        code_version: env!("CARGO_PKG_VERSION"),
    };
    let n = action(&ctx)?;
    println!("{n}");
    Ok(0)
}

/// Write `status.md` and `status.json` from the store. Needs no network.
///
/// The fidelity section is left empty: computing it needs the strategy, which
/// is built only inside the `run` command.
pub fn report() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
    let store = open_store(&cfg.run.db_path)?;
    let inputs = ReportInputs {
        store: &store,
        cfg: &cfg,
        stage: wiring::stage_name(owner.stage),
        fidelity: &[],
        validation_signoff: owner.validation_signoff.as_deref(),
        now: Utc::now(),
    };
    let (md, json) = write_status(&inputs)?;
    println!("wrote {} and {}", md.display(), json.display());
    Ok(0)
}

/// Exit 0 when the latest finished `ok` run is at most `max_age_hours` old.
/// Exit 1, with the reason in a log line, when there is none or it is older.
/// Without a limit, only the existence of an `ok` run is checked.
pub fn status(max_age_hours: Option<u64>) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store(&cfg.run.db_path)?;
    match last_ok_run_age_hours(&store, Utc::now())? {
        None => {
            wiring::log("error", "no successful run has finished");
            Ok(1)
        }
        Some(age) => {
            match max_age_hours {
                Some(max) if age > max as f64 => {
                    wiring::log(
                    "error",
                    &format!("last successful run finished {age:.1} hours ago, over the limit of {max}"),
                );
                    Ok(1)
                }
                _ => {
                    println!("last successful run finished {age:.1} hours ago");
                    Ok(0)
                }
            }
        }
    }
}

/// Compute the fidelity summary, judge it and write `signoff.md`. Needs no
/// network. Prints the verdict and exits 0, whatever the verdict.
///
/// With no stored calendar nothing has been recorded, so the sample is empty
/// and the verdict is `NOT_READY`.
pub fn signoff(action: &SignoffCmd) -> anyhow::Result<i32> {
    match action {
        SignoffCmd::Report => signoff_report(),
    }
}

fn signoff_report() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store(&cfg.run.db_path)?;
    let f = match store.calendar() {
        // No calendar means nothing has been stored: the sample is empty.
        Err(StoreError::NotFound(_)) => Fidelity {
            n_sessions: 0,
            n_rebalances: 0,
            n_fills: 0,
            replay_mismatches: 0,
            reconciliation_breaks: 0,
            missed_sessions: 0,
            late_rebalances: 0,
            mean_slippage_bps: 0.0,
            mean_diff_bps: 0.0,
            max_abs_diff_bps: 0.0,
        },
        Err(e) => return Err(e.into()),
        Ok(calendar) => {
            let params = TsmomParams {
                lookback_months: cfg.strategy.lookback_months.clone(),
                risk_free_symbol: Symbol::new(&cfg.strategy.risk_free_symbol)
                    .context("strategy.risk_free_symbol")?,
            };
            let strategy = Tsmom::new(params, Arc::new(calendar));
            fidelity(&store, &cfg, &strategy)?
        }
    };
    let report = evaluate(&f, &cfg);
    let out_dir = Path::new(&cfg.run.output_dir);
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let path = write_signoff(&report, out_dir)?;
    let word = match report.verdict {
        Verdict::Pass => "PASS",
        Verdict::Fail => "FAIL",
        Verdict::NotReady => "NOT_READY",
    };
    println!("{word}");
    println!("wrote {}", path.display());
    Ok(0)
}
