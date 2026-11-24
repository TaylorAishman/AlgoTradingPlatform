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

// R-003: every package, including all dependencies, has a licence and none is GPL or SSPL.
#[test]
fn every_package_has_a_permissive_licence() {
    let meta = metadata();
    for pkg in packages(&meta) {
        let name = pkg["name"].as_str().unwrap_or("?");
        let license = pkg["license"].as_str().unwrap_or("");
        let has_file = pkg["license_file"].as_str().is_some_and(|s| !s.is_empty());
        assert!(
            !license.is_empty() || has_file,
            "package {name} has no licence"
        );
        // An SPDX `OR` expression is satisfied by any one permissive alternative.
        let usable = license.is_empty()
            || license
                .split(" OR ")
                .any(|alt| !alt.contains("GPL") && !alt.contains("SSPL"));
        assert!(usable, "package {name} has a forbidden licence: {license}");
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
