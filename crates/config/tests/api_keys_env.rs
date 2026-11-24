//! R-041: API keys load from the environment map first.

use std::collections::BTreeMap;

use pt_config::error::ConfigError;
use pt_config::secrets::load_api_keys;

fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// R-041
#[test]
fn keys_come_from_the_environment_map() {
    let keys = load_api_keys(&env(&[
        ("APCA_API_KEY_ID", "K1"),
        ("APCA_API_SECRET_KEY", "S1"),
    ]))
    .expect("keys load");
    assert_eq!(keys.key_id.expose(), "K1");
    assert_eq!(keys.secret_key.expose(), "S1");
}

// R-041
#[test]
fn environment_wins_over_a_secrets_file_path() {
    let keys = load_api_keys(&env(&[
        ("APCA_API_KEY_ID", "K1"),
        ("APCA_API_SECRET_KEY", "S1"),
        ("PT_SECRETS_FILE", "/nonexistent/fake-secrets.toml"),
    ]))
    .expect("environment is consulted first");
    assert_eq!(keys.key_id.expose(), "K1");
    assert_eq!(keys.secret_key.expose(), "S1");
}

// R-041
#[test]
fn an_empty_map_is_missing_credentials() {
    let err = load_api_keys(&BTreeMap::new()).unwrap_err();
    assert_eq!(err, ConfigError::MissingCredentials);
}

// R-041
#[test]
fn only_one_of_the_two_variables_is_missing_credentials() {
    let err = load_api_keys(&env(&[("APCA_API_KEY_ID", "K1")])).unwrap_err();
    assert_eq!(err, ConfigError::MissingCredentials);
}
