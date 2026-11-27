//! R-550: every flag a command offers is one the design lists.

mod common;

use common::{stdout, trader};

const DESIGNED: [&str; 20] = [
    "--config",
    "--help",
    "--version",
    "--csv",
    "--adjustment",
    "--source",
    "--from",
    "--to",
    "--strategy",
    "--with-costs",
    "--capital",
    "--run",
    "--runs",
    "--benchmark-run",
    "--reason",
    "--id",
    "--note",
    "--yes",
    "--max-age-hours",
    "--if-due",
];

/// Every token matching `--[a-z][a-z-]*`.
fn flag_tokens(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < b.len() {
        if b[i] == b'-' && b[i + 1] == b'-' && b[i + 2].is_ascii_lowercase() {
            let mut j = i + 2;
            while j < b.len() && (b[j].is_ascii_lowercase() || b[j] == b'-') {
                j += 1;
            }
            out.push(text[i..j].to_string());
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Subcommand names listed under `Commands:`, without the generated `help`.
fn subcommands(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_commands = false;
    for line in text.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
            continue;
        }
        if !in_commands {
            continue;
        }
        if !line.starts_with("  ") {
            break;
        }
        if let Some(name) = line.split_whitespace().next() {
            if name != "help" {
                names.push(name.to_string());
            }
        }
    }
    names
}

fn walk(path: &[String], seen: &mut Vec<(Vec<String>, String)>) {
    let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
    args.push("--help");
    let out = trader(&args);
    assert_eq!(out.status.code(), Some(0), "{args:?} --help failed");
    let text = stdout(&out);
    seen.push((path.to_vec(), text.clone()));
    for sub in subcommands(&text) {
        let mut next = path.to_vec();
        next.push(sub);
        walk(&next, seen);
    }
}

fn all_help() -> Vec<(Vec<String>, String)> {
    let mut seen = Vec::new();
    walk(&[], &mut seen);
    seen
}

#[test]
fn every_flag_in_every_help_text_is_a_designed_flag() {
    // R-550
    let seen = all_help();
    assert!(seen.len() > 20, "walked only {} help texts", seen.len());
    for (path, text) in &seen {
        for flag in flag_tokens(text) {
            assert!(
                DESIGNED.contains(&flag.as_str()),
                "`trader {} --help` offers undesigned flag {flag}:\n{text}",
                path.join(" ")
            );
        }
    }
}

#[test]
fn run_help_offers_if_due() {
    // R-550
    let text = stdout(&trader(&["run", "--help"]));
    assert!(text.contains("--if-due"), "{text}");
}

#[test]
fn backtest_run_help_offers_capital() {
    // R-550
    let text = stdout(&trader(&["backtest", "run", "--help"]));
    assert!(text.contains("--capital"), "{text}");
}
