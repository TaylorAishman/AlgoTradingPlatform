//! R-397: the report and the heartbeat check run on their own, offline.

mod common;

use chrono::{Duration, Utc};
use common::{stderr, stdout};
use std::path::PathBuf;
use std::process::{Command, Output};

const CONFIG: &str = r#"
[data]
start = "2024-01-02"

[strategy]
lookback_months = [1]
risk_free_symbol = "BIL"
vol_min_obs = 5
vol_max_obs = 20

[sizing]
max_weight = 0.5

[risk]
max_order_notional = 60000

[[universe]]
symbol = "SPY"
asset_class = "equity"
shortable = true
"#;

struct Env {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    cfg: PathBuf,
    owner: PathBuf,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(
        &cfg,
        format!(
            "{CONFIG}\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            dir.path().join("test.sqlite").display(),
            dir.path().join("reports").display()
        ),
    )
    .unwrap();
    let owner = home.path().join("owner.toml");
    std::fs::write(&owner, "stage = \"ingest\"\n").unwrap();
    Env {
        dir,
        home,
        cfg,
        owner,
    }
}

fn run(e: &Env, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args(args)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("PT_OWNER_FILE", &e.owner)
        .env("APCA_API_KEY_ID", "fake-key-id")
        .env("APCA_API_SECRET_KEY", "fake-secret")
        .env("PT_ALPACA_TRADING_URL", "http://127.0.0.1:1")
        .env("PT_ALPACA_DATA_URL", "http://127.0.0.1:1")
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .output()
        .unwrap()
}

fn store(e: &Env) -> pt_store::db::Store {
    pt_store::db::Store::open(&e.dir.path().join("test.sqlite")).unwrap()
}

/// Records a run that ended with `status`, finished `hours_ago` hours ago.
fn seed_run(e: &Env, status: &str, hours_ago: i64) {
    let s = store(e);
    let finished = Utc::now() - Duration::hours(hours_ago);
    let id = s
        .start_run("ingest", "hash", "test", finished - Duration::minutes(1))
        .unwrap();
    s.finish_run(id, status, "", finished).unwrap();
}

fn reports(e: &Env) -> PathBuf {
    e.dir.path().join("reports")
}

fn not_a_stub(out: &Output) {
    assert!(
        !stderr(out).contains("not implemented"),
        "stderr: {}",
        stderr(out)
    );
}

#[test]
fn report_exits_zero_and_writes_status_md_and_status_json() {
    // R-397
    let e = env();
    let out = run(&e, &["report"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    not_a_stub(&out);
    let md = std::fs::read_to_string(reports(&e).join("status.md")).expect("status.md written");
    assert!(md.contains("Status"), "{md}");
    let text = std::fs::read_to_string(reports(&e).join("status.json")).expect("status.json");
    let json: serde_json::Value = serde_json::from_str(&text).expect("status.json is JSON");
    assert_eq!(json["stage"], "ingest");
}

#[test]
fn report_lists_an_active_halt_in_status_json() {
    // R-397
    let e = env();
    store(&e).set_halt("manual", "holiday", Utc::now()).unwrap();
    let out = run(&e, &["report"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = std::fs::read_to_string(reports(&e).join("status.json")).expect("status.json");
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let halts = json["active_halts"].as_array().expect("active_halts");
    assert_eq!(halts.len(), 1, "{json}");
    assert_eq!(halts[0]["detail"], "holiday");
}

#[test]
fn status_exits_one_when_there_is_no_successful_run() {
    // R-397
    let e = env();
    let out = run(&e, &["status", "--max-age-hours", "24"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    not_a_stub(&out);
}

#[test]
fn status_exits_one_when_the_only_runs_failed() {
    // R-397
    let e = env();
    seed_run(&e, "failed", 0);
    let out = run(&e, &["status", "--max-age-hours", "24"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    not_a_stub(&out);
}

#[test]
fn status_exits_zero_right_after_a_successful_run() {
    // R-397
    let e = env();
    seed_run(&e, "ok", 0);
    let out = run(&e, &["status", "--max-age-hours", "24"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
}

#[test]
fn status_exits_one_when_the_last_successful_run_is_older_than_the_limit() {
    // R-397
    let e = env();
    seed_run(&e, "ok", 48);
    let out = run(&e, &["status", "--max-age-hours", "24"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    not_a_stub(&out);
    // The same run is fresh enough under a wider limit, so age is what decides.
    let wide = run(&e, &["status", "--max-age-hours", "72"]);
    assert_eq!(wide.status.code(), Some(0), "stderr: {}", stderr(&wide));
}

#[test]
fn report_on_an_empty_store_has_ten_zero_fidelity_elements() {
    // R-539
    let e = env();
    let out = run(&e, &["report"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = std::fs::read_to_string(reports(&e).join("status.json")).expect("status.json");
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let fidelity = json["fidelity"].as_array().expect("fidelity array");
    assert_eq!(fidelity.len(), 10, "{json}");
    for f in fidelity {
        let zero = match &f["value"] {
            serde_json::Value::String(s) => s.parse::<f64>().ok() == Some(0.0),
            serde_json::Value::Number(n) => n.as_f64() == Some(0.0),
            _ => false,
        };
        assert!(zero, "element not zero: {f}");
    }
}
