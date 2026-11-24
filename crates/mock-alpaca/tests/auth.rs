//! R-111: the mock rejects unauthenticated requests.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

#[test]
fn account_without_key_id_is_401() {
    // R-111
    let mock = MockAlpaca::start();
    let resp = agent()
        .get(&format!("{}/v2/account", mock.url()))
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 401);
}

#[test]
fn account_without_any_auth_header_is_401() {
    // R-111
    let mock = MockAlpaca::start();
    let resp = agent()
        .get(&format!("{}/v2/account", mock.url()))
        .call()
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 401);
}

#[test]
fn account_without_secret_key_is_401() {
    // R-111
    let mock = MockAlpaca::start();
    let resp = agent()
        .get(&format!("{}/v2/account", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .call()
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 401);
}

#[test]
fn request_with_both_headers_is_not_rejected_as_unauthenticated() {
    // R-111
    let mock = MockAlpaca::start();
    let resp = agent()
        .get(&format!("{}/v2/account", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    assert_ne!(resp.status().as_u16(), 401);
}

#[test]
fn unauthenticated_request_is_still_recorded() {
    // R-111, R-110
    let mock = MockAlpaca::start();
    agent()
        .get(&format!("{}/v2/account", mock.url()))
        .call()
        .expect("the request reaches the mock");
    let reqs = mock.requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].path, "/v2/account");
}
