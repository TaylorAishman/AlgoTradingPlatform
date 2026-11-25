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
