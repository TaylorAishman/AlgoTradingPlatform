//! R-467: the first-order approval must state the quantity to send.

use std::path::Path;

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::owner::{load_owner_file, OwnerFile};

fn load(contents: &str) -> Result<OwnerFile, GateError> {
    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("owner.toml");
    std::fs::write(&path, contents).unwrap();
    load_owner_file(Path::new(&path), repo.path())
}

fn with_qty_line(qty_line: &str) -> String {
    format!(
        "stage = \"first_order\"\n\n[first_order]\nsymbol = \"SPY\"\nside = \"buy\"\n{qty_line}\nmax_qty = 5\nvalid_on = 2026-12-02\n"
    )
}

#[test]
fn qty_is_read_from_the_first_order_table() {
    // R-467
    let owner = load(&with_qty_line("qty = 2")).expect("loads");
    let approval = owner.first_order.expect("first_order table");
    assert_eq!(approval.qty, 2);
    assert_eq!(approval.max_qty, 5);
    assert_eq!(approval.symbol, "SPY");
    assert_eq!(approval.side, "buy");
    assert_eq!(
        approval.valid_on,
        NaiveDate::from_ymd_opt(2026, 12, 2).unwrap()
    );
}

#[test]
fn missing_qty_is_a_parse_error_naming_qty() {
    // R-467
    match load(&with_qty_line("")) {
        Err(GateError::Parse(msg)) => assert!(msg.contains("qty"), "message: {msg}"),
        other => panic!("expected Parse naming qty, got {other:?}"),
    }
}

#[test]
fn qty_zero_is_a_parse_error() {
    // R-467
    let err = load(&with_qty_line("qty = 0")).unwrap_err();
    assert!(matches!(err, GateError::Parse(_)), "got {err:?}");
}

#[test]
fn qty_above_max_qty_is_a_parse_error() {
    // R-467
    let err = load(&with_qty_line("qty = 6")).unwrap_err();
    assert!(matches!(err, GateError::Parse(_)), "got {err:?}");
}

#[test]
fn qty_equal_to_max_qty_loads() {
    // R-467
    let owner = load(&with_qty_line("qty = 5")).expect("loads");
    assert_eq!(owner.first_order.expect("table").qty, 5);
}

#[test]
fn qty_one_loads() {
    // R-467 lower boundary
    let owner = load(&with_qty_line("qty = 1")).expect("loads");
    assert_eq!(owner.first_order.expect("table").qty, 1);
}
