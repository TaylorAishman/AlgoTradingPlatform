//! Policy test: every declared dependency is used (R-403).

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// True when `text` has `name::` or `use name` as a whole path segment.
fn mentions(text: &str, name: &str) -> bool {
    let path_use = format!("{name}::");
    let use_decl = format!("use {name}");
    let path_hit = text
        .match_indices(&path_use)
        .any(|(i, _)| text[..i].chars().next_back().is_none_or(|c| !is_ident(c)));
    let use_hit = text.match_indices(&use_decl).any(|(i, _)| {
        text[i + use_decl.len()..]
            .chars()
            .next()
            .is_none_or(|c| !is_ident(c))
    });
    path_hit || use_hit
}

fn sources(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    rs_files(dir, &mut files);
    files
        .iter()
        .map(|f| fs::read_to_string(f).expect("read source"))
        .collect()
}

/// Names under `table` of the manifest that no source text mentions.
fn unused(manifest: &toml::Value, table: &str, texts: &[String]) -> Vec<String> {
    let Some(deps) = manifest.get(table).and_then(|t| t.as_table()) else {
        return Vec::new();
    };
    deps.keys()
        .filter(|k| {
            let name = k.replace('-', "_");
            !texts.iter().any(|t| mentions(t, &name))
        })
        .cloned()
        .collect()
}

#[test]
fn every_declared_dependency_is_used() {
    let mut problems = Vec::new();
    let mut crates: Vec<PathBuf> = fs::read_dir(root().join("crates"))
        .expect("read crates")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.join("Cargo.toml").is_file())
        .collect();
    crates.sort();
    assert!(!crates.is_empty(), "no crates found");
    for dir in crates {
        let name = dir.file_name().expect("name").to_string_lossy().to_string();
        let text = fs::read_to_string(dir.join("Cargo.toml")).expect("read manifest");
        let manifest: toml::Value = toml::from_str(&text).expect("parse manifest");
        for dep in unused(&manifest, "dependencies", &sources(&dir.join("src"))) {
            problems.push(format!("crate {name}: dependency {dep} is unused in src/"));
        }
        for dep in unused(&manifest, "dev-dependencies", &sources(&dir.join("tests"))) {
            problems.push(format!(
                "crate {name}: dev-dependency {dep} is unused in tests/"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_scan_tells_a_used_name_from_an_unused_one() {
    let texts = ["use pt_core::Order;\nlet x = serde_json::json!(1);".to_string()];
    assert!(mentions(&texts[0], "pt_core"));
    assert!(mentions(&texts[0], "serde_json"));
    assert!(!mentions(&texts[0], "pt_stats"));
    assert!(!mentions("use pt_core_extra;", "pt_core"));
    assert!(!mentions("my_serde_json::x", "serde_json"));
}
