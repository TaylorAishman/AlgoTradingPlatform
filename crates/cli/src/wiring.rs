//! Shared wiring for the command modules.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{RateLimiter, Sleeper, SystemClock, ThreadSleeper};
use pt_config::config::Config;
use pt_config::error::ConfigError;
use pt_config::secrets::{load_api_keys, ApiKeys};
use pt_gate::error::GateError;
use pt_gate::owner::{load_owner_file, OwnerFile, Stage};

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

/// The owner-file spelling of a stage.
pub fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Backtest => "backtest",
        Stage::Ingest => "ingest",
        Stage::DryRun => "dry_run",
        Stage::FirstOrder => "first_order",
        Stage::Paper => "paper",
    }
}

/// Check the owner's stage against a command's minimum stage. Returns the
/// refusal message when the stage is too low. Reads only the owner file.
pub fn stage_refusal(command: &str, required: Stage) -> anyhow::Result<Option<String>> {
    let current = load_owner()?.stage;
    if current >= required {
        return Ok(None);
    }
    Ok(Some(format!(
        "`{command}` needs stage `{}` or later; the owner file grants stage `{}`",
        stage_name(required),
        stage_name(current)
    )))
}

/// Calls per minute the client allows itself. The Basic plan allows 200
/// historical data requests per minute; architecture.md (line 82) says the
/// client limits itself to 180, leaving headroom.
const REQUESTS_PER_MINUTE: u32 = 180;

/// Load the API keys from the environment or `PT_SECRETS_FILE`. A failure names
/// the variables to set and never includes a key value.
pub fn load_keys() -> anyhow::Result<ApiKeys> {
    let env: BTreeMap<String, String> =
        ["APCA_API_KEY_ID", "APCA_API_SECRET_KEY", "PT_SECRETS_FILE"]
            .iter()
            .filter_map(|name| std::env::var(name).ok().map(|v| ((*name).to_string(), v)))
            .collect();
    load_api_keys(&env).map_err(|err| match err {
        ConfigError::MissingCredentials => anyhow!(
            "API keys are missing: set both APCA_API_KEY_ID and APCA_API_SECRET_KEY, \
             or set PT_SECRETS_FILE to a TOML file with key_id and secret_key"
        ),
        other => anyhow!("API keys could not be loaded from PT_SECRETS_FILE: {other}"),
    })
}

/// Load the keys and build the Alpaca client. The hosts come from
/// `pt_alpaca::endpoints`; `PT_ALPACA_TRADING_URL` and `PT_ALPACA_DATA_URL`
/// override them only for loopback hosts.
pub fn build_client() -> anyhow::Result<HttpClient> {
    let keys = load_keys()?;
    let trading = std::env::var("PT_ALPACA_TRADING_URL").ok();
    let data = std::env::var("PT_ALPACA_DATA_URL").ok();
    let endpoints = if trading.is_some() || data.is_some() {
        Endpoints::with_overrides(trading.as_deref(), data.as_deref())?
    } else {
        Endpoints::production()
    };
    let creds = Credentials::new(
        keys.key_id.expose().to_string(),
        keys.secret_key.expose().to_string(),
    );
    let sleeper: Arc<dyn Sleeper> = Arc::new(ThreadSleeper);
    let limiter = RateLimiter::new(
        REQUESTS_PER_MINUTE,
        Arc::new(SystemClock::new()),
        sleeper.clone(),
    );
    Ok(HttpClient::new(endpoints, creds, limiter, sleeper))
}

/// Write one JSON log line to stderr. Stdout is kept for command results.
pub fn log(level: &str, message: &str) {
    let line = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "level": level,
        "message": message,
    });
    eprintln!("{line}");
}
