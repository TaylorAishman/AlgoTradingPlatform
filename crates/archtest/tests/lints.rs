//! Policy tests for workspace lints (R-007).

use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn manifest(path: &std::path::Path) -> toml::Table {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .parse()
        .unwrap_or_else(|e| panic!("invalid TOML in {}: {e}", path.display()))
}

// R-007: unsafe code is forbidden at the workspace root.
#[test]
fn root_manifest_forbids_unsafe_code() {
    let doc = manifest(&root().join("Cargo.toml"));
    let level = doc["workspace"]["lints"]["rust"]["unsafe_code"].as_str();
    assert_eq!(level, Some("forbid"));
}

// R-007: every crate manifest inherits the workspace lints.
#[test]
fn every_crate_manifest_inherits_workspace_lints() {
    let mut checked = 0;
    for entry in fs::read_dir(root().join("crates")).expect("crates dir") {
        let dir = entry.expect("dir entry").path();
        let path = dir.join("Cargo.toml");
        if !path.is_file() {
            continue;
        }
        let doc = manifest(&path);
        let inherits = doc
            .get("lints")
            .and_then(|l| l.get("workspace"))
            .and_then(|w| w.as_bool());
        assert_eq!(
            inherits,
            Some(true),
            "{} lacks lints.workspace = true",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 0, "no crate manifests found");
}

// R-007: the scaffolded foundation crates exist and inherit the lints.
#[test]
fn foundation_crates_exist_and_inherit_workspace_lints() {
    for dir in ["core", "calendar", "stats", "vector"] {
        let path = root().join("crates").join(dir).join("Cargo.toml");
        assert!(path.is_file(), "missing {}", path.display());
        let doc = manifest(&path);
        let inherits = doc
            .get("lints")
            .and_then(|l| l.get("workspace"))
            .and_then(|w| w.as_bool());
        assert_eq!(inherits, Some(true), "crates/{dir} lacks lints.workspace");
    }
}
