//! `data` commands (stub).

use crate::cli::DataCmd;

pub fn data(_action: &DataCmd) -> anyhow::Result<i32> {
    anyhow::bail!("`data` is not implemented yet")
}
