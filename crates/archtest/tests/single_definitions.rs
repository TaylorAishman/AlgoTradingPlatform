//! Policy tests that shared helpers have one definition (R-404 to R-407).

use std::fs;
use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Every `.rs` file under `dir`, with its text.
fn sources(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    collect_rs(dir, &mut paths);
    assert!(!paths.is_empty(), "no files under {}", dir.display());
    paths
        .into_iter()
        .map(|p| {
            let text = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (p, text)
        })
        .collect()
}

fn cli_src() -> Vec<(PathBuf, String)> {
    sources(&crates_dir().join("cli").join("src"))
}

fn files_containing(files: &[(PathBuf, String)], needle: &str) -> Vec<String> {
    files
        .iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(p, _)| p.display().to_string())
        .collect()
}

fn assert_in_exactly_one_file(needle: &str) {
    let found = files_containing(&cli_src(), needle);
    assert_eq!(
        found.len(),
        1,
        "`{needle}` must occur in exactly one file under crates/cli/src, found in {found:?}"
    );
}

// R-404: no .rs file under crates/*/src contains `fn stage_name`.
#[test]
fn stage_name_has_no_definition_in_any_crate_source() {
    let mut found = Vec::new();
    for entry in fs::read_dir(crates_dir()).expect("crates dir") {
        let src = entry.expect("dir entry").path().join("src");
        if src.is_dir() {
            found.extend(files_containing(&sources(&src), "fn stage_name"));
        }
    }
    assert!(found.is_empty(), "`fn stage_name` found in {found:?}");
}

// R-405: `fn open_store` occurs in exactly one file under crates/cli/src.
#[test]
fn store_opener_is_defined_in_one_cli_file() {
    assert_in_exactly_one_file("fn open_store");
}

// R-405: `PT_ALERT_WEBHOOK_URL` occurs in exactly one file under crates/cli/src.
#[test]
fn webhook_lookup_is_in_one_cli_file() {
    assert_in_exactly_one_file("PT_ALERT_WEBHOOK_URL");
}

// R-405: `impl Strategy for` occurs in exactly one file under crates/cli/src.
#[test]
fn stored_strategy_wrapper_is_in_one_cli_file() {
    assert_in_exactly_one_file("impl Strategy for");
}

// R-406: neither strategy id appears in any file under crates/cli/src.
#[test]
fn cli_sources_do_not_repeat_strategy_ids() {
    let files = cli_src();
    for id in ["tsmom-v1", "bh-volscaled-v1"] {
        let found = files_containing(&files, id);
        assert!(found.is_empty(), "`{id}` found in {found:?}");
    }
}

// R-407: no file under crates/cli/src contains `env::args`.
#[test]
fn cli_sources_do_not_read_the_process_arguments() {
    let found = files_containing(&cli_src(), "env::args");
    assert!(found.is_empty(), "`env::args` found in {found:?}");
}
