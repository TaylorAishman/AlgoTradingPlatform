//! R-041: API keys load from the file named by `PT_SECRETS_FILE` when the
//! environment map has no keys.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use pt_config::error::ConfigError;
use pt_config::secrets::load_api_keys;

fn write_secrets(dir: &Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("secrets.toml");
    std::fs::write(&path, body).expect("write secrets file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod 0600");
    path
}

fn env_with_file(path: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert(
        "PT_SECRETS_FILE".to_string(),
        path.to_str().expect("utf8 path").to_string(),
    );
    map
}

// R-041
#[test]
fn keys_come_from_a_mode_0600_secrets_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_secrets(dir.path(), "key_id = \"K2\"\nsecret_key = \"S2\"\n");
    let keys = load_api_keys(&env_with_file(&path)).expect("keys load from file");
    assert_eq!(keys.key_id.expose(), "K2");
    assert_eq!(keys.secret_key.expose(), "S2");
}

// R-041
#[test]
fn environment_keys_win_over_the_secrets_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_secrets(dir.path(), "key_id = \"K2\"\nsecret_key = \"S2\"\n");
    let mut env = env_with_file(&path);
    env.insert("APCA_API_KEY_ID".to_string(), "K1".to_string());
    env.insert("APCA_API_SECRET_KEY".to_string(), "S1".to_string());
    let keys = load_api_keys(&env).expect("keys load");
    assert_eq!(keys.key_id.expose(), "K1");
    assert_eq!(keys.secret_key.expose(), "S1");
}

// R-041
#[test]
fn loaded_file_keys_do_not_print() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_secrets(dir.path(), "key_id = \"K2\"\nsecret_key = \"S2\"\n");
    let keys = load_api_keys(&env_with_file(&path)).expect("keys load from file");
    let shown = format!("{:?}", keys);
    assert!(!shown.contains("K2") && !shown.contains("S2"));
}

// R-041
#[test]
fn no_environment_keys_and_no_file_is_missing_credentials() {
    let err = load_api_keys(&BTreeMap::new()).unwrap_err();
    assert_eq!(err, ConfigError::MissingCredentials);
}
