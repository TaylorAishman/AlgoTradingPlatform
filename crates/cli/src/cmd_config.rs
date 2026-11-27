//! `config`, `gates`, `calendar` and `universe` commands.

use crate::cli::{CalendarCmd, ConfigCmd, UniverseCmd};
use chrono::NaiveDate;
use pt_core::types::{Adjustment, Symbol};
use pt_gate::status::{gate_status, Facts};
use pt_store::db::Store;

use crate::wiring;

pub fn config(action: &ConfigCmd) -> anyhow::Result<i32> {
    match action {
        ConfigCmd::Check => {
            let cfg = wiring::load_config()?;
            println!("{}", pt_config::hash::config_hash(&cfg));
            Ok(0)
        }
    }
}

/// Bounds for "any stored bar". The store compares dates as text, so they
/// must be plain four-digit years (`NaiveDate::MAX` formats as `+262142-...`).
const EARLIEST: NaiveDate = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
const LATEST: NaiveDate = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();

/// What the store already holds: a calendar, and raw bars for every universe
/// symbol on the configured feed. Read offline; a missing config or store
/// counts as nothing loaded, and a store that does not exist is not created.
fn store_facts() -> (bool, bool) {
    let Ok(cfg) = wiring::load_config() else {
        return (false, false);
    };
    let path = std::path::Path::new(&cfg.run.db_path);
    if !path.is_file() {
        return (false, false);
    }
    let Ok(store) = Store::open(path) else {
        return (false, false);
    };
    let calendar_loaded = store.calendar().is_ok();
    let bars_loaded = !cfg.universe.is_empty()
        && cfg.universe.iter().all(|inst| {
            Symbol::new(&inst.symbol)
                .ok()
                .and_then(|sym| {
                    store
                        .bars_asof(
                            &sym,
                            Adjustment::Raw,
                            &cfg.data.feed,
                            EARLIEST,
                            LATEST,
                            None,
                        )
                        .ok()
                })
                .is_some_and(|bars| !bars.is_empty())
        });
    (calendar_loaded, bars_loaded)
}

/// Print the stage and the owner checklist. Needs no network: keys count when
/// they load from the environment or `PT_SECRETS_FILE`, and the calendar and
/// bars count when the configured store holds them. The first-order fact is
/// not yet read.
pub fn gates() -> anyhow::Result<i32> {
    let owner = wiring::load_owner()?;
    let keys_present = wiring::load_keys().is_ok();
    let (calendar_loaded, bars_loaded) = store_facts();
    let facts = Facts {
        keys_present,
        calendar_loaded,
        bars_loaded,
        first_order_sent: false,
    };
    println!("stage: {}", owner.stage.as_str());
    for item in gate_status(&owner, &facts) {
        if item.done {
            println!("{}: done", item.id);
        } else {
            println!("{}: pending - {}", item.id, item.instruction);
        }
    }
    Ok(0)
}

pub fn calendar(action: &CalendarCmd) -> anyhow::Result<i32> {
    match action {
        CalendarCmd::Import { csv } => crate::cmd_data::calendar_import(csv),
        CalendarCmd::Sync => crate::cmd_data::calendar_sync(),
    }
}

pub fn universe(action: &UniverseCmd) -> anyhow::Result<i32> {
    match action {
        UniverseCmd::Verify => crate::cmd_data::universe_verify(),
    }
}
