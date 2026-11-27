//! `halt`, `report`, `status` and `signoff` commands.

use std::path::Path;

use anyhow::Context;
use chrono::Utc;
use pt_core::types::Symbol;
use pt_ops::report::last_ok_run_age_hours;
use pt_shadow::fidelity::fidelity;
use pt_shadow::signoff::{evaluate, write_signoff, Verdict};
use pt_store::error::StoreError;
use pt_strategy::tsmom::TsmomParams;

use crate::cli::{HaltCmd, SignoffCmd};
use crate::reporting;
use crate::strategies::StoredTsmom;
use crate::wiring;

/// Inspect or change halts. No stage is required and no request is made.
///
/// `status` exits 0 with no active halt and 2 with one; `set` and `clear`
/// exit 0, and `clear` of an unknown or already cleared id exits 1.
pub fn halt(action: &HaltCmd) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = wiring::open_store(&cfg.run.db_path)?;
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

/// Write `status.md` and `status.json` from the store. Needs no network.
///
/// The report inputs, fidelity summary included, are assembled in
/// `reporting.rs`, shared with `run`.
pub fn report() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
    let store = wiring::open_store(&cfg.run.db_path)?;
    let (md, json) = reporting::write_report(
        &store,
        &cfg,
        owner.stage.as_str(),
        owner.validation_signoff.as_deref(),
    )?;
    println!("wrote {} and {}", md.display(), json.display());
    Ok(0)
}

/// Exit 0 when the latest finished `ok` run is at most `max_age_hours` old.
/// Exit 1, with the reason in a log line, when there is none or it is older.
/// Without a limit, only the existence of an `ok` run is checked.
pub fn status(max_age_hours: Option<u64>) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = wiring::open_store(&cfg.run.db_path)?;
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
/// and the verdict is `NOT_READY`; the fidelity summary handles that itself.
pub fn signoff(action: &SignoffCmd) -> anyhow::Result<i32> {
    match action {
        SignoffCmd::Report => signoff_report(),
    }
}

fn signoff_report() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = wiring::open_store(&cfg.run.db_path)?;
    let params = TsmomParams {
        lookback_months: cfg.strategy.lookback_months.clone(),
        risk_free_symbol: Symbol::new(&cfg.strategy.risk_free_symbol)
            .context("strategy.risk_free_symbol")?,
    };
    let strategy = StoredTsmom::new(&store, params, cfg.data.start)?;
    let f = fidelity(&store, &cfg, &strategy)?;
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
