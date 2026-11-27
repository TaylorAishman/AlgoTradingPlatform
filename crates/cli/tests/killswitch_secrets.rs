//! R-533: the kill-switch commands print no secret. Neither stdout nor stderr
//! of `cancel-open --yes`, `flatten --yes` (success and rejection) or of
//! either command without `--yes` contains the key id or the secret key.

mod common;

use common::live::{env, Env};
use common::{stderr, stdout};

const KEY_ID: &str = "AKTESTKEYID7f3a9c1e";
const SECRET: &str = "sEcReT-7f3a9c1e-d84b2a60";

fn run_with_keys(e: &Env, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args(args)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("PT_OWNER_FILE", &e.owner)
        .env("PT_ALPACA_TRADING_URL", e.mock.url())
        .env("PT_ALPACA_DATA_URL", e.mock.url())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env("APCA_API_KEY_ID", KEY_ID)
        .env("APCA_API_SECRET_KEY", SECRET)
        .output()
        .unwrap()
}

fn assert_no_secret(out: &std::process::Output, what: &str) {
    for (name, text) in [("stdout", stdout(out)), ("stderr", stderr(out))] {
        assert!(!text.contains(KEY_ID), "{what} {name} leaks key id: {text}");
        assert!(!text.contains(SECRET), "{what} {name} leaks secret: {text}");
    }
}

fn ingest_with_positions(e: &Env) {
    std::fs::write(&e.owner, "stage = \"ingest\"\n").unwrap();
    e.mock.seed_position("SPY", 10);
    e.mock.seed_position("TLT", -5);
}

#[test]
fn cancel_open_with_yes_prints_no_secret() {
    // R-533
    let e = env();
    let out = run_with_keys(&e, &["cancel-open", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert_no_secret(&out, "cancel-open --yes");
}

#[test]
fn flatten_with_yes_prints_no_secret_on_success() {
    // R-533
    let e = env();
    ingest_with_positions(&e);
    let out = run_with_keys(&e, &["flatten", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert_no_secret(&out, "flatten --yes");
}

#[test]
fn flatten_with_yes_prints_no_secret_when_an_order_is_rejected() {
    // R-533
    let e = env();
    ingest_with_positions(&e);
    e.mock.reject_next("POST", "/v2/orders", 1, 422);
    let out = run_with_keys(&e, &["flatten", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );
    assert_no_secret(&out, "flatten --yes rejected");
}

#[test]
fn kill_switch_commands_without_yes_print_no_secret() {
    // R-533
    for cmd in ["cancel-open", "flatten"] {
        let e = env();
        let out = run_with_keys(&e, &[cmd]);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{cmd}: {}{}",
            stdout(&out),
            stderr(&out)
        );
        assert_no_secret(&out, cmd);
    }
}
