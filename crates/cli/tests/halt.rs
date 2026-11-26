//! R-395: halts are inspectable and changeable by the owner.

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

#[test]
fn halt_status_with_no_halts_prints_no_active_halts_and_exits_zero() {
    // R-395
    let e = env();
    let out = run(&e, &["halt", "status"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("no active halts"), "{}", stdout(&out));
}

#[test]
fn halt_set_then_status_shows_id_reason_and_detail_and_exits_two() {
    // R-395
    let e = env();
    let set = run(&e, &["halt", "set", "--reason", "holiday"]);
    assert_eq!(set.status.code(), Some(0), "stderr: {}", stderr(&set));

    let out = run(&e, &["halt", "status"]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("no active halts"), "{text}");
    assert!(text.contains("manual"), "{text}");
    assert!(text.contains("holiday"), "{text}");
    assert!(
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|t| t == "1"),
        "halt id 1 missing: {text}"
    );
}

#[test]
fn halt_clear_makes_status_exit_zero_again() {
    // R-395
    let e = env();
    assert_eq!(
        run(&e, &["halt", "set", "--reason", "holiday"])
            .status
            .code(),
        Some(0)
    );
    let clear = run(&e, &["halt", "clear", "--id", "1", "--note", "back"]);
    assert_eq!(clear.status.code(), Some(0), "stderr: {}", stderr(&clear));

    let out = run(&e, &["halt", "status"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("no active halts"), "{}", stdout(&out));
}

#[test]
fn halt_clear_of_an_unknown_id_exits_one() {
    // R-395
    let e = env();
    let out = run(&e, &["halt", "clear", "--id", "99", "--note", "x"]);
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout(&out));
    assert!(
        !stderr(&out).contains("not implemented"),
        "stderr: {}",
        stderr(&out)
    );
}
