//! Policy tests for dependency licences (R-003).

use std::path::PathBuf;
use std::process::Command;

fn metadata() -> serde_json::Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(root)
        .output()
        .expect("run cargo metadata");
    assert!(out.status.success(), "cargo metadata failed");
    serde_json::from_slice(&out.stdout).expect("metadata JSON")
}

fn packages(meta: &serde_json::Value) -> &Vec<serde_json::Value> {
    meta["packages"].as_array().expect("packages array")
}

/// Packages that carry a licence file and no SPDX expression (empty today).
const FILE_LICENSED: &[&str] = &[];

/// A part is allowed when it names neither a GPL family licence nor SSPL.
fn part_allowed(part: &str) -> bool {
    let p = part.trim();
    !p.is_empty() && !p.contains("GPL") && !p.contains("SSPL")
}

/// Parses `or_expr := and_expr (OR and_expr)*` over tokens; `/` reads as `OR`.
fn parse_or(tokens: &[String], pos: &mut usize) -> bool {
    let mut any = parse_and(tokens, pos);
    while *pos < tokens.len() && tokens[*pos] == "OR" {
        *pos += 1;
        let next = parse_and(tokens, pos);
        any = any || next;
    }
    any
}

/// Parses `and_expr := atom (AND atom)*`.
fn parse_and(tokens: &[String], pos: &mut usize) -> bool {
    let mut all = parse_atom(tokens, pos);
    while *pos < tokens.len() && tokens[*pos] == "AND" {
        *pos += 1;
        let next = parse_atom(tokens, pos);
        all = all && next;
    }
    all
}

/// Parses `atom := '(' or_expr ')' | name`.
fn parse_atom(tokens: &[String], pos: &mut usize) -> bool {
    match tokens.get(*pos).map(String::as_str) {
        Some("(") => {
            *pos += 1;
            let inner = parse_or(tokens, pos);
            if tokens.get(*pos).map(String::as_str) == Some(")") {
                *pos += 1;
            }
            inner
        }
        Some(name) if name != ")" && name != "OR" && name != "AND" => {
            *pos += 1;
            // `X WITH exception` is judged by X alone.
            if tokens.get(*pos).map(String::as_str) == Some("WITH") {
                *pos += 2;
            }
            part_allowed(name)
        }
        _ => false,
    }
}

/// True when the SPDX expression has an allowed alternative (F-27).
fn licence_allowed(expr: &str) -> bool {
    let spaced = expr
        .replace('(', " ( ")
        .replace(')', " ) ")
        .replace('/', " OR ");
    let tokens: Vec<String> = spaced.split_whitespace().map(str::to_string).collect();
    if tokens.is_empty() {
        return false;
    }
    let mut pos = 0;
    let ok = parse_or(&tokens, &mut pos);
    ok && pos == tokens.len()
}

// R-003: expressions with an allowed alternative are accepted.
#[test]
fn licence_allowed_accepts_expressions_with_a_permissive_alternative() {
    for expr in [
        "MIT",
        "MIT OR GPL-3.0",
        "MIT/Apache-2.0",
        "(MIT OR Apache-2.0) AND Unicode-3.0",
        "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    ] {
        assert!(licence_allowed(expr), "{expr} should be allowed");
    }
}

// R-003: copyleft-only expressions and the empty string are refused.
#[test]
fn licence_allowed_refuses_copyleft_and_empty() {
    for expr in [
        "GPL-3.0-only",
        "LGPL-2.1",
        "AGPL-3.0",
        "SSPL-1.0",
        "MIT AND GPL-2.0",
        "",
    ] {
        assert!(!licence_allowed(expr), "{expr:?} should be refused");
    }
}

// R-003: every package, including all dependencies, has an allowed licence expression,
// or no expression, a licence file and a place on the explicit list.
#[test]
fn every_package_has_a_permissive_licence() {
    let meta = metadata();
    for pkg in packages(&meta) {
        let name = pkg["name"].as_str().unwrap_or("?");
        let license = pkg["license"].as_str().unwrap_or("");
        let has_file = pkg["license_file"].as_str().is_some_and(|s| !s.is_empty());
        if license.is_empty() {
            assert!(
                has_file && FILE_LICENSED.contains(&name),
                "package {name} has no licence expression and is not a listed file-licensed package"
            );
        } else {
            assert!(
                licence_allowed(license),
                "package {name} has a forbidden licence: {license}"
            );
        }
    }
}

// R-003: the check covers the scaffolded pt-config, pt-gate and pt-store packages.
#[test]
fn licence_check_covers_config_gate_and_store() {
    let meta = metadata();
    let names: Vec<&str> = packages(&meta)
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    for want in ["pt-config", "pt-gate", "pt-store"] {
        assert!(names.contains(&want), "{want} is not in the workspace");
    }
}
