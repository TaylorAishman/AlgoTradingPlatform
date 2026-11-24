//! Policy test for the workspace member list (R-001).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

// R-001: the workspace contains exactly the twenty packages of Arch section 4.
#[test]
fn workspace_has_exactly_the_twenty_packages() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .expect("run cargo metadata");
    assert!(out.status.success(), "cargo metadata failed");
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).expect("metadata JSON");
    let actual: BTreeSet<String> = meta["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .map(|p| p["name"].as_str().expect("package name").to_string())
        .collect();
    let expected: BTreeSet<String> = [
        "pt-archtest",
        "pt-core",
        "pt-calendar",
        "pt-config",
        "pt-store",
        "pt-alpaca",
        "pt-mock-alpaca",
        "pt-data",
        "pt-strategy",
        "pt-portfolio",
        "pt-risk",
        "pt-engine",
        "pt-vector",
        "pt-stats",
        "pt-gate",
        "pt-live",
        "pt-shadow",
        "pt-ops",
        "pt-validate",
        "pt-cli",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(actual, expected);
}
