//! Shared helpers for the `trader` binary tests.
#![allow(dead_code)]

use std::process::{Command, Output};

/// Runs the built `trader` binary with a scrubbed environment, so no real
/// configuration, owner file or key can leak in.
pub fn trader(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .args(args)
        .env_remove("PT_CONFIG")
        .env_remove("PT_OWNER_FILE")
        .env_remove("PT_SECRETS_FILE")
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .output()
        .expect("run trader binary")
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

pub mod live;
