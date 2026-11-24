//! R-110: the mock listens on loopback and records every request.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn port_of(url: &str) -> u16 {
    url.rsplit(':')
        .next()
        .unwrap()
        .trim_end_matches('/')
        .parse()
        .expect("url ends with a numeric port")
}

#[test]
fn url_is_on_loopback() {
    // R-110
    let mock = MockAlpaca::start();
    assert!(
        mock.url().starts_with("http://127.0.0.1:"),
        "url was {}",
        mock.url()
    );
}

#[test]
fn records_method_path_and_query_of_a_request() {
    // R-110
    let mock = MockAlpaca::start();
    assert!(mock.requests().is_empty());
    agent()
        .get(&format!("{}/v2/clock?x=1", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    let reqs = mock.requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[0].path, "/v2/clock");
    assert_eq!(reqs[0].query, "x=1");
}

#[test]
fn two_mocks_listen_on_different_ports() {
    // R-110
    let a = MockAlpaca::start();
    let b = MockAlpaca::start();
    assert_ne!(port_of(&a.url()), port_of(&b.url()));
}
