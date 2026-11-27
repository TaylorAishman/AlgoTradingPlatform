//! Policy tests for shared test fixtures and corrected comments (R-408 to R-411).

use std::fs;
use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn read(rel: &str) -> String {
    let path = crates_dir().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn files_defining(dir: &str, signature: &str) -> Vec<String> {
    let root = crates_dir().join(dir);
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    assert!(!files.is_empty(), "no files under {dir}");
    let mut found = Vec::new();
    for f in files {
        let text = fs::read_to_string(&f).expect("read file");
        if text.contains(signature) {
            let rel = f.strip_prefix(&root).expect("under root");
            found.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    found
}

fn assert_single_shared(signature: &str) {
    let found = files_defining("cli/tests", signature);
    assert_eq!(found.len(), 1, "`{signature}` defined in {found:?}");
    assert!(
        found[0].starts_with("common/"),
        "`{signature}` defined in {found:?}, not under common/"
    );
}

// R-408
#[test]
fn calendar_sessions_is_defined_once_in_the_shared_test_module() {
    assert_single_shared("fn calendar_sessions");
}

// R-408
#[test]
fn bars_csv_is_defined_once_in_the_shared_test_module() {
    assert_single_shared("fn bars_csv");
}

// R-409
#[test]
fn no_cli_source_file_calls_anything_a_stub() {
    let mut files = Vec::new();
    rs_files(&crates_dir().join("cli/src"), &mut files);
    assert!(!files.is_empty(), "no files scanned");
    let hits: Vec<_> = files
        .iter()
        .filter(|f| {
            fs::read_to_string(f)
                .expect("read file")
                .to_lowercase()
                .contains("stub")
        })
        .collect();
    assert!(hits.is_empty(), "stale word in {hits:?}");
}

// R-410
#[test]
fn adv_shares_doc_states_the_window_as_on_or_before() {
    let text = read("core/src/view.rs");
    let lines: Vec<&str> = text.lines().collect();
    let idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with("fn adv_shares") && l.trim_end().ends_with(';'))
        .expect("trait method adv_shares");
    let mut doc = Vec::new();
    let mut i = idx;
    while i > 0 && lines[i - 1].trim_start().starts_with("///") {
        doc.push(lines[i - 1]);
        i -= 1;
    }
    let doc = doc.join("\n");
    assert!(!doc.is_empty(), "adv_shares has no doc comment");
    assert!(doc.contains("on or before"), "doc: {doc}");
    assert!(!doc.contains("sessions before"), "doc: {doc}");
}

// R-411
#[test]
fn store_ops_module_comment_names_what_the_file_holds() {
    let text = read("store/src/ops.rs");
    let head: String = text
        .lines()
        .take_while(|l| l.starts_with("//!"))
        .collect::<Vec<_>>()
        .join(" ");
    for word in ["runs", "halts", "alerts", "trials", "backtests"] {
        assert!(head.contains(word), "missing `{word}` in: {head}");
    }
}
