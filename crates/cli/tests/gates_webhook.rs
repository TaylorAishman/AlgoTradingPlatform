//! R-531: the checklist's webhook item sees the environment variable the alerter uses.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const URL: &str = "http://127.0.0.1:9/hook";

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

fn gates(dir: &Path, home: &Path, cfg: &Path, webhook: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trader"));
    cmd.arg("--config")
        .arg(cfg)
        .arg("gates")
        .current_dir(dir)
        .env("HOME", home)
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .env_remove("PT_OWNER_FILE")
        .env_remove("PT_ALERT_WEBHOOK_URL");
    if let Some(url) = webhook {
        cmd.env("PT_ALERT_WEBHOOK_URL", url);
    }
    cmd.output().unwrap()
}

fn webhook_line(out: &Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    text.lines()
        .find(|l| l.starts_with("alert_webhook"))
        .unwrap_or_else(|| panic!("no alert_webhook line in: {text}"))
        .to_string()
}

#[test]
fn the_environment_variable_marks_alert_webhook_done() {
    // R-531
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let line = webhook_line(&gates(dir.path(), home.path(), &cfg, Some(URL)));
    assert!(line.contains("done"), "line: {line}");
}

#[test]
fn neither_owner_file_nor_environment_leaves_alert_webhook_pending() {
    // R-531
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let line = webhook_line(&gates(dir.path(), home.path(), &cfg, None));
    assert!(line.contains("pending"), "line: {line}");
}

#[test]
fn the_webhook_url_does_not_appear_in_the_output() {
    // R-531
    let home = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path());
    let out = gates(dir.path(), home.path(), &cfg, Some(URL));
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!all.contains("127.0.0.1:9"), "output: {all}");
    assert!(!all.contains("/hook"), "output: {all}");
}
