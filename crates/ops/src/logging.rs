//! Structured logging: each event is one JSON line.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::json;

/// Format a log event as a single-line JSON object with `ts`, `level`,
/// `message` and `run_id` (null when absent).
pub fn format_event(level: &str, message: &str, run_id: Option<i64>, ts: DateTime<Utc>) -> String {
    json!({
        "ts": ts.to_rfc3339_opts(SecondsFormat::Secs, true),
        "level": level,
        "message": message,
        "run_id": run_id,
    })
    .to_string()
}
