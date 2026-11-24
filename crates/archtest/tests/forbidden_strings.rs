//! Policy tests for forbidden strings in source files (R-005).

use std::fs;
use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs") | Some("toml")
        ) {
            out.push(path);
        }
    }
}

fn scanned_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(crates_dir()).expect("crates dir") {
        let dir = entry.expect("dir entry").path();
        if dir.is_dir() && dir.file_name().is_some_and(|n| n != "archtest") {
            collect(&dir, &mut files);
        }
    }
    files
}

// R-005: no .rs or .toml file under crates/ (outside archtest) names the live trading host.
#[test]
fn no_source_file_names_the_live_trading_host() {
    let needle = format!("//api.{}.markets", "alpaca");
    let files = scanned_files();
    assert!(!files.is_empty(), "no files scanned");
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        assert!(
            !text.contains(&needle),
            "{} names the live trading host",
            file.display()
        );
    }
}

// R-005: the scan covers the scaffolded pt-alpaca, pt-mock-alpaca and pt-data crates.
#[test]
fn scan_covers_alpaca_mock_alpaca_and_data_crates() {
    let files = scanned_files();
    for (dir, name) in [
        ("alpaca", "pt-alpaca"),
        ("mock-alpaca", "pt-mock-alpaca"),
        ("data", "pt-data"),
    ] {
        let manifest = crates_dir().join(dir).join("Cargo.toml");
        let canon = fs::canonicalize(&manifest)
            .unwrap_or_else(|e| panic!("missing {}: {e}", manifest.display()));
        assert!(
            files
                .iter()
                .any(|f| fs::canonicalize(f).is_ok_and(|c| c == canon)),
            "{name} manifest is not scanned"
        );
        let text = fs::read_to_string(&manifest).expect("read manifest");
        assert!(
            text.contains(&format!("name = \"{name}\"")),
            "{name} has the wrong package name"
        );
    }
}
