//! R-381: the config is checkable from the command line.

mod common;

use common::{stderr, stdout, trader};
use std::process::{Command, Output};

const VALID: &str = include_str!("../../../config/paper-trader.example.toml");

fn write_config(dir: &tempfile::TempDir, extra: &str) -> std::path::PathBuf {
    let path = dir.path().join("cfg.toml");
    std::fs::write(&path, format!("{VALID}\n{extra}\n")).unwrap();
    path
}

fn run_with_env_config(path: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .args(["config", "check"])
        .env("PT_CONFIG", path)
        .env_remove("PT_OWNER_FILE")
        .env_remove("PT_SECRETS_FILE")
        .output()
        .unwrap()
}

fn has_64_hex_token(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))
}

#[test]
fn valid_config_exits_zero_and_prints_the_64_char_hash() {
    // R-381
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "");
    let out = trader(&["--config", path.to_str().unwrap(), "config", "check"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(has_64_hex_token(&stdout(&out)), "stdout: {}", stdout(&out));
}

#[test]
fn valid_config_path_from_pt_config_is_accepted() {
    // R-381
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "");
    let out = run_with_env_config(&path);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(has_64_hex_token(&stdout(&out)));
}

#[test]
fn the_hash_is_stable_across_runs() {
    // R-381
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "");
    let a = trader(&["--config", path.to_str().unwrap(), "config", "check"]);
    let b = trader(&["--config", path.to_str().unwrap(), "config", "check"]);
    assert_eq!(stdout(&a), stdout(&b));
}

#[test]
fn max_gross_of_two_exits_one_and_names_max_gross() {
    // R-381
    let dir = tempfile::tempdir().unwrap();
    let path = write_config(&dir, "[sizing]\nmax_gross = 2");
    let out = trader(&["--config", path.to_str().unwrap(), "config", "check"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("max_gross"),
        "stderr: {}",
        stderr(&out)
    );
    assert!(!has_64_hex_token(&stdout(&out)));
}

#[test]
fn no_config_path_and_no_pt_config_exits_one_naming_pt_config() {
    // R-381
    let out = trader(&["config", "check"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("PT_CONFIG"),
        "stderr: {}",
        stderr(&out)
    );
}

fn cli_sources() -> Vec<(String, String)> {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    std::fs::read_dir(src)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .map(|p| {
            (
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

#[test]
fn no_source_file_reads_env_args() {
    // R-407
    for (path, text) in cli_sources() {
        assert!(!text.contains("env::args"), "{path} contains env::args");
    }
}

#[test]
fn the_command_line_is_parsed_in_exactly_one_place() {
    // R-407: the config path is resolved once, by the argument parser.
    let calls: Vec<String> = cli_sources()
        .into_iter()
        .filter(|(_, t)| t.contains("Cli::parse("))
        .map(|(p, _)| p)
        .collect();
    let total: usize = cli_sources()
        .iter()
        .map(|(_, t)| t.matches("Cli::parse(").count())
        .sum();
    assert_eq!(total, 1, "Cli::parse( appears in {calls:?}");
}
