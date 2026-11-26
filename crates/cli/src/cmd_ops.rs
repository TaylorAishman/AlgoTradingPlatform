//! `halt`, `cancel-open`, `flatten`, `report`, `status` and `signoff` commands (stubs).

use std::path::Path;

use anyhow::Context;
use chrono::Utc;
use pt_store::db::Store;
use pt_store::error::StoreError;

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

pub fn cancel_open(_args: &ConfirmArgs) -> anyhow::Result<i32> {
    anyhow::bail!("`cancel-open` is not implemented yet")
}

pub fn flatten(_args: &ConfirmArgs) -> anyhow::Result<i32> {
    anyhow::bail!("`flatten` is not implemented yet")
}

pub fn report() -> anyhow::Result<i32> {
    anyhow::bail!("`report` is not implemented yet")
}

pub fn status(_max_age_hours: Option<u64>) -> anyhow::Result<i32> {
    anyhow::bail!("`status` is not implemented yet")
}

pub fn signoff(_action: &SignoffCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`signoff` is not implemented yet")
}
