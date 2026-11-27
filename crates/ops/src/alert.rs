//! Alerts: every alert is logged and stored, and posted once to an optional
//! webhook. A slow or failing webhook never blocks or fails the caller.

use crate::logging::format_event;
use chrono::{DateTime, SecondsFormat, Utc};
use pt_store::db::Store;
use serde_json::json;
use std::time::Duration;

/// Longest the whole webhook request may take before it is abandoned.
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(5);

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
        let message = self.scrub(message);
        let message = message.as_str();
        let code = self.scrub(code);
        let code = code.as_str();
        eprintln!("{}", format_event(level.as_str(), message, run_id, at));
        let delivered = self.post(level, code, message, run_id, at);
        if let Err(e) = store.record_alert(level.as_str(), code, message, delivered, at) {
            eprintln!(
                "{}",
                format_event("error", &format!("alert not stored: {e}"), run_id, at)
            );
        }
        delivered
    }

    /// Replace every occurrence of each secret string with `***`.
    fn scrub(&self, text: &str) -> String {
        let mut out = text.to_string();
        for secret in self.secrets.iter().filter(|s| !s.is_empty()) {
            out = out.replace(secret.as_str(), "***");
        }
        out
    }

    /// POST the alert as JSON to the webhook; true when it answered with success.
    fn post(
        &self,
        level: Level,
        code: &str,
        message: &str,
        run_id: Option<i64>,
        at: DateTime<Utc>,
    ) -> bool {
        let Some(url) = &self.webhook else {
            return false;
        };
        let body = json!({
            "ts": at.to_rfc3339_opts(SecondsFormat::Secs, true),
            "level": level.as_str(),
            "code": code,
            "message": message,
            "run_id": run_id,
        })
        .to_string();
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(WEBHOOK_TIMEOUT))
            .max_redirects(0)
            .build()
            .into();
        agent
            .post(url)
            .header("Content-Type", "application/json")
            .send(body)
            .is_ok_and(|r| r.status().is_success())
    }
}
