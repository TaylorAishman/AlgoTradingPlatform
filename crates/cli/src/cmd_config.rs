//! `config`, `gates`, `calendar` and `universe` commands (stubs).

use crate::cli::{CalendarCmd, ConfigCmd, UniverseCmd};
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

pub fn gates() -> anyhow::Result<i32> {
    anyhow::bail!("`gates` is not implemented yet")
}

pub fn calendar(_action: &CalendarCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`calendar` is not implemented yet")
}

pub fn universe(_action: &UniverseCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`universe` is not implemented yet")
}
