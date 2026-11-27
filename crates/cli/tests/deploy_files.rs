//! Example schedules under `deploy/` (R-400).

use std::fs;
use std::path::PathBuf;

fn deploy_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../deploy")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Text with the plist DOCTYPE line removed (it legitimately names an Apple DTD URL).
fn without_doctype(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("<!DOCTYPE"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_no_secrets(name: &str, text: &str) {
    let body = without_doctype(text);
    let lower = body.to_lowercase();
    assert!(
        !lower.contains("http://") && !lower.contains("https://"),
        "{name} contains a URL"
    );
    assert!(!lower.contains("hooks."), "{name} contains a webhook host");
    for needle in ["apca_api_secret_key=", "apca_api_key_id=", "webhook_url="] {
        assert!(!lower.contains(needle), "{name} assigns {needle}");
    }
}

/// Non-comment, non-blank lines that run `trader run`.
fn cron_entries(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter(|l| l.contains("trader") && l.split_whitespace().any(|w| w == "run"))
        .map(|l| l.split_whitespace().map(String::from).collect())
        .collect()
}

// R-400
#[test]
fn cron_example_has_no_cron_tz_line() {
    let text = deploy_file("cron.example");
    assert!(
        !text.lines().any(|l| l.trim_start().starts_with("CRON_TZ")),
        "cron.example must not set CRON_TZ"
    );
}

// R-400
#[test]
fn cron_example_has_one_entry_every_fifteen_minutes_running_if_due() {
    let text = deploy_file("cron.example");
    let entries = cron_entries(&text);
    assert_eq!(entries.len(), 1, "expected one schedule entry: {entries:?}");
    let e = &entries[0];
    assert!(e.len() >= 7, "entry too short: {e:?}");
    assert_eq!(&e[..5], ["*/15", "*", "*", "*", "*"], "time fields: {e:?}");
    let pos = e
        .iter()
        .position(|w| w.rsplit('/').next() == Some("trader"))
        .expect("no trader command");
    assert_eq!(e[pos + 1], "run", "entry: {e:?}");
    assert_eq!(e[pos + 2], "--if-due", "entry: {e:?}");
}

// R-400
#[test]
fn cron_example_names_new_york_time() {
    assert!(deploy_file("cron.example").contains("America/New_York"));
}

// R-400
#[test]
fn cron_example_contains_no_key_or_webhook_url() {
    assert_no_secrets("cron.example", &deploy_file("cron.example"));
}

/// Minimal well-formedness check: tags nest and close, one root element.
fn assert_well_formed(xml: &str) {
    let mut stack: Vec<String> = Vec::new();
    let mut roots = 0;
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        let after = &rest[i..];
        if let Some(r) = after.strip_prefix("<!--") {
            let end = r.find("-->").expect("unterminated comment");
            rest = &r[end + 3..];
            continue;
        }
        if after.starts_with("<?") {
            let end = after.find("?>").expect("unterminated declaration");
            rest = &after[end + 2..];
            continue;
        }
        if after.starts_with("<!") {
            let end = after.find('>').expect("unterminated doctype");
            rest = &after[end + 1..];
            continue;
        }
        let end = after.find('>').expect("unterminated tag");
        let inner = &after[1..end];
        rest = &after[end + 1..];
        if let Some(name) = inner.strip_prefix('/') {
            let open = stack.pop().expect("closing tag with nothing open");
            assert_eq!(open, name.trim(), "mismatched closing tag");
        } else {
            let self_closing = inner.ends_with('/');
            let name = inner
                .trim_end_matches('/')
                .split_whitespace()
                .next()
                .expect("empty tag")
                .to_string();
            if stack.is_empty() {
                roots += 1;
            }
            if !self_closing {
                stack.push(name);
            }
        }
    }
    assert!(stack.is_empty(), "unclosed tags: {stack:?}");
    assert_eq!(roots, 1, "expected exactly one root element");
}

const PLIST: &str = "launchd/com.paper-trader.run.plist.example";

// R-400
#[test]
fn launchd_example_is_well_formed_xml() {
    let text = deploy_file(PLIST);
    assert!(
        text.trim_start().starts_with("<?xml"),
        "missing XML declaration"
    );
    assert_well_formed(&text);
}

// R-400
#[test]
fn launchd_example_runs_trader_run() {
    let text = deploy_file(PLIST);
    let start = text
        .find("<key>ProgramArguments</key>")
        .expect("missing ProgramArguments");
    let after = &text[start..];
    let arr_start = after
        .find("<array>")
        .expect("ProgramArguments has no array");
    let arr_end = after
        .find("</array>")
        .expect("ProgramArguments array unclosed");
    let args: Vec<&str> = after[arr_start..arr_end]
        .split("<string>")
        .skip(1)
        .map(|s| s.split("</string>").next().unwrap().trim())
        .collect();
    assert!(
        args.iter().any(|a| a.rsplit('/').next() == Some("trader")),
        "no trader program argument: {args:?}"
    );
    assert!(args.contains(&"run"), "no run program argument: {args:?}");
    assert!(
        args.contains(&"--if-due"),
        "no --if-due program argument: {args:?}"
    );
}

// R-400
#[test]
fn launchd_example_names_new_york_time() {
    assert!(deploy_file(PLIST).contains("America/New_York"));
}

// R-400
#[test]
fn launchd_example_has_four_quarter_hour_intervals_without_hour() {
    let text = deploy_file(PLIST);
    let start = text
        .find("<key>StartCalendarInterval</key>")
        .expect("missing StartCalendarInterval");
    let after = &text[start..];
    let arr_start = after
        .find("<array>")
        .expect("StartCalendarInterval is not an array");
    let arr_end = after
        .find("</array>")
        .expect("StartCalendarInterval array unclosed");
    let block = &after[arr_start..arr_end];
    assert_eq!(
        block.matches("<dict>").count(),
        4,
        "expected four intervals"
    );
    assert_eq!(block.matches("<key>Hour</key>").count(), 0, "Hour is set");
    let mut minutes: Vec<u32> = block
        .split("<key>Minute</key>")
        .skip(1)
        .map(|s| {
            s.split("<integer>")
                .nth(1)
                .and_then(|t| t.split("</integer>").next())
                .and_then(|t| t.trim().parse().ok())
                .expect("Minute has an integer value")
        })
        .collect();
    minutes.sort();
    assert_eq!(minutes, vec![0, 15, 30, 45]);
}

// R-400
#[test]
fn launchd_example_contains_no_key_or_webhook_url() {
    assert_no_secrets(PLIST, &deploy_file(PLIST));
}
