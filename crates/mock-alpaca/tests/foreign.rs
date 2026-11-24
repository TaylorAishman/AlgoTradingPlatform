//! R-118: the mock exposes activity it did not receive through the API.

use pt_mock_alpaca::server::MockAlpaca;

fn get_json(mock: &MockAlpaca, path: &str) -> serde_json::Value {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(&format!("{}{}", mock.url(), path))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    let text = resp.body_mut().read_to_string().expect("body");
    serde_json::from_str(&text).expect("json")
}

#[test]
fn a_seeded_foreign_order_appears_in_the_order_list() {
    // R-118
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    mock.seed_foreign_order("manual-1", "SPY", "buy", 1);
    let orders = get_json(&mock, "/v2/orders?status=all");
    let found = orders
        .as_array()
        .expect("array")
        .iter()
        .find(|o| o["client_order_id"] == "manual-1")
        .expect("the foreign order is listed");
    assert_eq!(found["symbol"], "SPY");
    assert_eq!(found["side"], "buy");
}

#[test]
fn a_seeded_foreign_position_appears_in_the_position_list() {
    // R-118
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_position("GLD", 3);
    let positions = get_json(&mock, "/v2/positions");
    let found = positions
        .as_array()
        .expect("array")
        .iter()
        .find(|p| p["symbol"] == "GLD")
        .expect("the foreign position is listed");
    assert_eq!(found["qty"], "3");
}

#[test]
fn nothing_foreign_is_listed_unless_seeded() {
    // R-118 boundary
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    assert!(get_json(&mock, "/v2/orders?status=all")
        .as_array()
        .expect("array")
        .is_empty());
    assert!(get_json(&mock, "/v2/positions")
        .as_array()
        .expect("array")
        .is_empty());
}

#[test]
fn a_seeded_foreign_order_has_an_rfc3339_utc_submitted_at() {
    // R-118
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_foreign_order("manual-1", "SPY", "buy", 1);
    mock.seed_foreign_order("manual-2", "SPY", "buy", 1);
    let at = |id: &str| {
        let v = get_json(
            &mock,
            &format!("/v2/orders:by_client_order_id?client_order_id={id}"),
        );
        let text = v["submitted_at"].as_str().expect("a string").to_string();
        assert!(text.ends_with('Z'), "UTC: {text}");
        chrono::DateTime::parse_from_rfc3339(&text).expect("RFC 3339")
    };
    assert!(at("manual-2") > at("manual-1"));
}
