//! `config`, `gates`, `calendar` and `universe` commands (stubs).

use crate::cli::{CalendarCmd, ConfigCmd, UniverseCmd};

pub fn config(_action: &ConfigCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`config` is not implemented yet")
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
