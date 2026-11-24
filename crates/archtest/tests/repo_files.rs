//! Policy tests for the repository root files (R-002, R-006).

use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(name: &str) -> String {
    fs::read_to_string(root().join(name)).unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
}

fn has_line(text: &str, wanted: &str) -> bool {
    text.lines().any(|l| l.trim() == wanted)
}

// R-002: toolchain pinned to 1.100.0 with rustfmt and clippy.
#[test]
fn toolchain_channel_is_pinned_to_1_100_0() {
    let doc: toml::Table = read("rust-toolchain.toml").parse().expect("valid TOML");
    let channel = doc["toolchain"]["channel"].as_str();
    assert_eq!(channel, Some("1.100.0"));
}

// R-002
#[test]
fn toolchain_includes_rustfmt_and_clippy() {
    let doc: toml::Table = read("rust-toolchain.toml").parse().expect("valid TOML");
    let components: Vec<&str> = doc["toolchain"]["components"]
        .as_array()
        .expect("components is an array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(components.contains(&"rustfmt"), "got {components:?}");
    assert!(components.contains(&"clippy"), "got {components:?}");
}

// R-006: build output and run-time data are ignored.
#[test]
fn gitignore_ignores_target_and_var() {
    let text = read(".gitignore");
    assert!(has_line(&text, "target/"), "missing target/");
    assert!(has_line(&text, "var/"), "missing var/");
}

// R-006: the existing PDF rule must survive.
#[test]
fn gitignore_still_ignores_reference_pdfs() {
    let text = read(".gitignore");
    assert!(has_line(&text, "docs/reference/trading-papers/*.pdf"));
}

// R-006: the lock file is not a text diff.
#[test]
fn gitattributes_marks_cargo_lock_as_no_diff() {
    let text = read(".gitattributes");
    assert!(has_line(&text, "Cargo.lock -diff"));
}
