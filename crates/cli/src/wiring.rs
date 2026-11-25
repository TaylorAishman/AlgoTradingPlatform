//! Shared wiring for the command modules.

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use pt_config::config::Config;

use clap::Parser;

use crate::cli::Cli;

/// The config path: `--config` first, then `PT_CONFIG`.
///
/// The dispatcher hands commands only their own action, so the global flag is
/// read again from the process arguments (already validated by the first parse).
pub fn config_path() -> anyhow::Result<PathBuf> {
    if let Some(path) = Cli::parse().config {
        return Ok(path);
    }
    match std::env::var_os("PT_CONFIG") {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => Err(anyhow!(
            "no config file: pass --config <PATH> or set PT_CONFIG"
        )),
    }
}

/// Resolve the config path, then read and validate the file.
pub fn load_config() -> anyhow::Result<Config> {
    let path = config_path()?;
    pt_config::config::load(&path).with_context(|| format!("config {}", path.display()))
}
