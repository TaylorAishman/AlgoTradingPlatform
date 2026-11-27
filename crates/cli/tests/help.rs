//! R-380: the binary lists its commands.

mod common;

use common::{stderr, stdout, trader};

#[test]
fn help_exits_zero_and_lists_every_command() {
    // R-380
    let out = trader(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    for cmd in [
        "config",
        "gates",
        "calendar",
        "data",
        "universe",
        "doctor",
        "backtest",
        "validate",
        "run",
        "halt",
        "cancel-open",
        "flatten",
        "report",
        "status",
        "signoff",
    ] {
        assert!(text.contains(cmd), "help output lacks `{cmd}`:\n{text}");
    }
}

#[test]
fn unknown_command_exits_two_with_usage() {
    // R-380
    let out = trader(&["frobnicate"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).to_lowercase().contains("usage"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn command_module_comments_do_not_call_anything_a_stub() {
    // R-409
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read source");
        checked += 1;
        assert!(
            !text.to_lowercase().contains("stub"),
            "{} contains the word stub",
            path.display()
        );
    }
    assert!(checked > 0, "no source files found under {}", dir.display());
}
