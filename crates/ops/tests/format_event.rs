//! R-354: a log event is one JSON line.

use chrono::{DateTime, TimeZone, Utc};
use pt_ops::logging::format_event;
use serde_json::Value;

fn t() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, 14, 30, 5).unwrap()
}

fn parse(line: &str) -> Value {
    serde_json::from_str(line).expect("event must parse as JSON")
}

#[test]
fn event_is_a_single_line_with_level_message_run_id_and_ts() {
    // R-354
    let line = format_event("warn", "hello \"x\"", Some(3), t());
    assert!(!line.contains('\n'), "must be one line: {line:?}");
    let v = parse(&line);
    assert_eq!(v["level"], "warn");
    assert_eq!(v["message"], "hello \"x\"");
    assert_eq!(v["run_id"], 3);
    let ts = DateTime::parse_from_rfc3339(v["ts"].as_str().expect("ts is a string"))
        .expect("ts is RFC 3339");
    assert_eq!(ts.with_timezone(&Utc), t());
}

#[test]
fn missing_run_id_is_json_null() {
    // R-354
    let v = parse(&format_event("info", "hi", None, t()));
    assert!(v.get("run_id").expect("run_id key present").is_null());
}

#[test]
fn message_with_newlines_stays_on_one_line_and_round_trips() {
    // R-354
    let msg = "line one\nline two\r\n\ttabbed \\ backslash";
    let line = format_event("error", msg, Some(1), t());
    assert!(!line.contains('\n') && !line.contains('\r'), "{line:?}");
    assert_eq!(parse(&line)["message"], msg);
}
