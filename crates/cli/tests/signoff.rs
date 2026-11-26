//! R-398: the sign-off report runs on its own, offline.

mod common;

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

[signoff]
min_sessions = 63

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

fn signoff_md(e: &Env) -> PathBuf {
    e.dir.path().join("reports").join("signoff.md")
}

#[test]
fn signoff_report_exits_zero_and_prints_not_ready_for_a_small_store() {
    // R-398
    let e = env();
    let out = run(&e, &["signoff", "report"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        stdout(&out).contains("NOT_READY"),
        "stdout: {}",
        stdout(&out)
    );
}

#[test]
fn signoff_report_writes_signoff_md_under_the_output_directory() {
    // R-398
    let e = env();
    let out = run(&e, &["signoff", "report"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let md = std::fs::read_to_string(signoff_md(&e)).expect("signoff.md written");
    assert!(md.contains("NOT READY"), "{md}");
    assert!(md.contains("replay_mismatches"), "{md}");
}

#[test]
fn signoff_report_can_be_run_again() {
    // R-398
    let e = env();
    for _ in 0..2 {
        let out = run(&e, &["signoff", "report"]);
        assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
        assert!(
            stdout(&out).contains("NOT_READY"),
            "stdout: {}",
            stdout(&out)
        );
    }
    assert!(signoff_md(&e).exists());
}
