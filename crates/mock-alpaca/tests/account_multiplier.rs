//! R-442: the mock broker's margin multiplier is settable.

use pt_mock_alpaca::server::MockAlpaca;

fn account(mock: &MockAlpaca) -> serde_json::Value {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(&format!("{}/v2/account", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 200);
    let text = resp.body_mut().read_to_string().expect("body is text");
    serde_json::from_str(&text).expect("body is JSON")
}

#[test]
fn account_multiplier_is_two_by_default() {
    // R-442
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    assert_eq!(account(&mock)["multiplier"], "2");
}

#[test]
fn seed_account_multiplier_changes_the_reported_multiplier() {
    // R-442
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_account_multiplier("1");
    assert_eq!(account(&mock)["multiplier"], "1");
}
