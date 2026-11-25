//! `halt`, `cancel-open`, `flatten`, `report`, `status` and `signoff` commands (stubs).

use crate::cli::{ConfirmArgs, HaltCmd, SignoffCmd};

pub fn halt(_action: &HaltCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`halt` is not implemented yet")
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
