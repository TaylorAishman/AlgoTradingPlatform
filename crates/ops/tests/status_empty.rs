//! Tests for reporting an empty store and a halted system (R-360).

use chrono::{DateTime, TimeZone, Utc};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use serde_json::Value;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, 22, 0, 0).unwrap()
}

fn config(dir: &std::path::Path) -> pt_config::config::Config {
    pt_config::config::parse(&format!(
        "[run]\noutput_dir = {:?}\n\n[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n",
        dir.to_str().unwrap()
    ))
    .unwrap()
}

fn report(store: &Store, cfg: &pt_config::config::Config) -> (std::path::PathBuf, Value) {
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity: &[],
        validation_signoff: None,
        now: now(),
    };
    let (md, json) = write_status(&inp).expect("write_status succeeds");
    let v = serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    (md, v)
}

// R-360: with an empty store the report is written with null equity and last_run, no positions.
#[test]
fn empty_store_reports_null_equity_no_positions_and_no_last_run() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let (md, v) = report(&store, &cfg);
    assert!(md.exists());
    assert!(v["equity"].is_null());
    assert_eq!(v["positions"].as_array().unwrap().len(), 0);
    assert!(v["last_run"].is_null());
    assert_eq!(v["active_halts"].as_array().unwrap().len(), 0);
}

// R-360: an active daily_loss halt appears in active_halts and the report is still written.
#[test]
fn active_daily_loss_halt_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    store.set_halt("daily_loss", "down 3%", now()).unwrap();
    let (md, v) = report(&store, &cfg);
    let halts = v["active_halts"].as_array().unwrap();
    assert_eq!(halts.len(), 1);
    assert_eq!(halts[0]["reason"], "daily_loss");
    assert!(std::fs::read_to_string(md).unwrap().contains("daily_loss"));
}
