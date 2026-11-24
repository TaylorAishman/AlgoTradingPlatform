//! R-042: a secrets file readable by group or others is refused.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use pt_config::error::ConfigError;
use pt_config::secrets::load_api_keys;

fn secrets_with_mode(dir: &Path, mode: u32) -> BTreeMap<String, String> {
    let path = dir.join("secrets.toml");
    std::fs::write(&path, "key_id = \"K2\"\nsecret_key = \"S2\"\n").expect("write secrets file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("chmod");
    let mut map = BTreeMap::new();
    map.insert(
        "PT_SECRETS_FILE".to_string(),
        path.to_str().expect("utf8 path").to_string(),
    );
    map
}

fn assert_refused(mode: u32) {
    let dir = tempfile::tempdir().expect("tempdir");
    let env = secrets_with_mode(dir.path(), mode);
    let err = load_api_keys(&env).unwrap_err();
    assert!(
        matches!(err, ConfigError::InsecureSecretsFile(_)),
        "mode {mode:o} gave {err:?}"
    );
}

// R-042
#[test]
fn mode_0644_secrets_file_is_refused() {
    assert_refused(0o644);
}

// R-042
#[test]
fn group_readable_secrets_file_is_refused() {
    assert_refused(0o640);
}

// R-042
#[test]
fn other_readable_secrets_file_is_refused() {
    assert_refused(0o604);
}

// R-042
#[test]
fn group_writable_secrets_file_is_refused() {
    assert_refused(0o620);
}

// R-042
#[test]
fn refusal_message_does_not_leak_key_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let env = secrets_with_mode(dir.path(), 0o644);
    let err = load_api_keys(&env).unwrap_err();
    let shown = format!("{err} {err:?}");
    assert!(matches!(err, ConfigError::InsecureSecretsFile(_)));
    assert!(!shown.contains("K2") && !shown.contains("S2"));
}

// R-042
#[test]
fn mode_0600_secrets_file_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let env = secrets_with_mode(dir.path(), 0o600);
    let keys = load_api_keys(&env).expect("0600 loads");
    assert_eq!(keys.key_id.expose(), "K2");
}

// R-042
#[test]
fn mode_0400_secrets_file_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let env = secrets_with_mode(dir.path(), 0o400);
    let keys = load_api_keys(&env).expect("0400 loads");
    assert_eq!(keys.secret_key.expose(), "S2");
}
