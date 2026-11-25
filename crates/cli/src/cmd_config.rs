//! `config`, `gates`, `calendar` and `universe` commands (stubs).

use crate::cli::{CalendarCmd, ConfigCmd, UniverseCmd};
use pt_gate::owner::Stage;
use pt_gate::status::{gate_status, Facts};

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

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Backtest => "backtest",
        Stage::Ingest => "ingest",
        Stage::DryRun => "dry_run",
        Stage::FirstOrder => "first_order",
        Stage::Paper => "paper",
    }
}

/// Print the stage and the owner checklist. Needs no keys and no network, so
/// the store-backed facts (calendar, bars, first order) are reported as not
/// yet seen.
pub fn gates() -> anyhow::Result<i32> {
    let owner = wiring::load_owner()?;
    let keys_present = ["APCA_API_KEY_ID", "APCA_API_SECRET_KEY"]
        .iter()
        .all(|name| std::env::var_os(name).is_some_and(|v| !v.is_empty()));
    let facts = Facts {
        keys_present,
        calendar_loaded: false,
        bars_loaded: false,
        first_order_sent: false,
    };
    println!("stage: {}", stage_name(owner.stage));
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

pub fn universe(_action: &UniverseCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`universe` is not implemented yet")
}
