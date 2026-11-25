//! Shared wiring for the command modules.

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use pt_config::config::Config;
use pt_gate::error::GateError;
use pt_gate::owner::{load_owner_file, OwnerFile};

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

/// The owner-file path: `PT_OWNER_FILE`, else `$HOME/.config/paper-trader/owner.toml`.
pub fn owner_file_path() -> anyhow::Result<PathBuf> {
    match std::env::var_os("PT_OWNER_FILE") {
        Some(path) if !path.is_empty() => Ok(PathBuf::from(path)),
        _ => match std::env::var_os("HOME") {
            Some(home) if !home.is_empty() => Ok(PathBuf::from(home)
                .join(".config")
                .join("paper-trader")
                .join("owner.toml")),
            _ => Err(anyhow!("no owner file: set PT_OWNER_FILE or HOME")),
        },
    }
}

/// The repository root: the nearest ancestor of the working directory that
/// contains `.git`, or the working directory itself.
pub fn repo_root() -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir().context("working directory")?;
    let root = cwd
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .unwrap_or(&cwd)
        .to_path_buf();
    Ok(root)
}

/// Load the owner file. A missing file means stage `backtest`; a file inside
/// the repository is refused.
pub fn load_owner() -> anyhow::Result<OwnerFile> {
    let path = owner_file_path()?;
    let root = repo_root()?;
    load_owner_file(&path, &root).map_err(|err| match err {
        GateError::OwnerFileInRepo => anyhow!(
            "owner file {} must be outside the repository {}",
            path.display(),
            root.display()
        ),
        other => anyhow!(other).context(format!("owner file {}", path.display())),
    })
}
