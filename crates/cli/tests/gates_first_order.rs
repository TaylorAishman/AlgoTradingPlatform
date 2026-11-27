//! R-530: the checklist shows whether the first order has been accepted.

use chrono::{NaiveDate, TimeZone, Utc};
use pt_core::order_state::OrderState;
use pt_core::types::{Side, Symbol};
use pt_store::db::Store;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn write_config(dir: &Path) -> PathBuf {
    let cfg = dir.join("cfg.toml");
    std::fs::write(
        &cfg,
        format!(
            "[[universe]]\nsymbol = \"S1\"\nasset_class = \"equity\"\nshortable = true\n\n\
             [data]\nfeed = \"csv:synth\"\n\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            dir.join("store.sqlite").display(),
            dir.join("reports").display()
        ),
    )
    .unwrap();
    cfg
}

fn gates(dir: &Path, home: &Path, cfg: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(cfg)
        .arg("gates")
        .current_dir(dir)
        .env("HOME", home)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("PT_OWNER_FILE")
        .env_remove("PT_ALERT_WEBHOOK_URL")
        .output()
        .unwrap()
}

fn first_order_line(out: &Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.lines()
        .find(|l| l.starts_with("first_order_sent"))
        .unwrap_or_else(|| panic!("no first_order_sent line in: {text}"))
        .to_string()
}

/// Stores one first-order row and moves it to `state` (Submitted or Rejected).
fn store_first_order(dir: &Path, state: OrderState) {
    let store = Store::open(&dir.join("store.sqlite")).unwrap();
    store
        .insert_order(&OrderRow {
            client_order_id: "pt-first-20240301-SPY-1".to_string(),
            decision_id: None,
            symbol: Symbol::new("SPY").unwrap(),
            side: Side::Buy,
            qty: 1,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::from(100),
            trade_date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
        })
        .unwrap();
    let at = Utc.with_ymd_and_hms(2024, 3, 1, 15, 0, 0).unwrap();
    let broker = (state == OrderState::Submitted).then_some("b-1");
    store
        .set_order_state("pt-first-20240301-SPY-1", state, 0, broker, at)
        .unwrap();
}

#[test]
fn a_submitted_first_order_marks_first_order_sent_done() {
    // R-530
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    store_first_order(dir.path(), OrderState::Submitted);
    let line = first_order_line(&gates(dir.path(), home.path(), &cfg));
    assert!(line.contains("done"), "line: {line}");
}

#[test]
fn a_rejected_first_order_leaves_first_order_sent_pending() {
    // R-530
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    store_first_order(dir.path(), OrderState::Rejected);
    let line = first_order_line(&gates(dir.path(), home.path(), &cfg));
    assert!(line.contains("pending"), "line: {line}");
}

#[test]
fn no_database_leaves_first_order_sent_pending_and_creates_no_file() {
    // R-530
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let line = first_order_line(&gates(dir.path(), home.path(), &cfg));
    assert!(line.contains("pending"), "line: {line}");
    assert!(
        !dir.path().join("store.sqlite").exists(),
        "gates created the store"
    );
}
