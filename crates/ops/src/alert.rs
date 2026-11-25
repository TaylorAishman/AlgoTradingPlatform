//! Alerts: every alert is logged and stored; delivery to a webhook comes later.

use crate::logging::format_event;
use chrono::{DateTime, Utc};
use pt_store::db::Store;

/// How serious an alert is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Critical,
}

impl Level {
    /// The lower-case name stored and logged for this level.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Critical => "critical",
        }
    }
}

/// Sends alerts to the log and the database, and optionally a webhook.
#[derive(Debug, Clone)]
pub struct Alerter {
    webhook: Option<String>,
    secrets: Vec<String>,
}

impl Alerter {
    /// Build an alerter with an optional webhook URL and secret strings.
    pub fn new(webhook: Option<String>, secrets: Vec<String>) -> Self {
        Self { webhook, secrets }
    }

    /// Log and store an alert. Returns whether it was delivered to a webhook.
    pub fn alert(
        &self,
        store: &Store,
        level: Level,
        code: &str,
        message: &str,
        run_id: Option<i64>,
        at: DateTime<Utc>,
    ) -> bool {
        let _ = (&self.webhook, &self.secrets);
        eprintln!("{}", format_event(level.as_str(), message, run_id, at));
        let delivered = false;
        if let Err(e) = store.record_alert(level.as_str(), code, message, delivered, at) {
            eprintln!(
                "{}",
                format_event("error", &format!("alert not stored: {e}"), run_id, at)
            );
        }
        delivered
    }
}
