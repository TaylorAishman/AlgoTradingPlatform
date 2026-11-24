//! R-115 (expire) and R-119 (set_order_status): the mock reports any order status.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn post_order(mock: &MockAlpaca, client_order_id: &str) {
    let body = serde_json::json!({
        "symbol": "SPY",
        "qty": "10",
        "side": "buy",
        "type": "market",
        "time_in_force": "day",
        "client_order_id": client_order_id,
    })
    .to_string();
    let resp = agent()
        .post(&format!("{}/v2/orders", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .header("Content-Type", "application/json")
        .send(body)
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 200);
}

fn get(mock: &MockAlpaca, path_and_query: &str) -> serde_json::Value {
    let mut resp = agent()
        .get(&format!("{}{}", mock.url(), path_and_query))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 200);
    let text = resp.body_mut().read_to_string().expect("body reads");
    serde_json::from_str(&text).expect("json body")
}

fn by_client_id(mock: &MockAlpaca, client_order_id: &str) -> serde_json::Value {
    get(
        mock,
        &format!("/v2/orders:by_client_order_id?client_order_id={client_order_id}"),
    )
}

fn listed(mock: &MockAlpaca, client_order_id: &str) -> serde_json::Value {
    get(mock, "/v2/orders?status=all")
        .as_array()
        .expect("a JSON array")
        .iter()
        .find(|o| o["client_order_id"] == client_order_id)
        .cloned()
        .expect("the order is listed")
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    post_order(&mock, "c1");
    mock
}

#[test]
fn expire_sets_an_unfilled_order_to_expired() {
    // R-115
    let mock = seeded();
    assert_eq!(by_client_id(&mock, "c1")["status"], "accepted");
    mock.expire("c1");
    assert_eq!(by_client_id(&mock, "c1")["status"], "expired");
    assert_eq!(listed(&mock, "c1")["status"], "expired");
}

#[test]
fn expire_leaves_positions_empty() {
    // R-115
    let mock = seeded();
    mock.expire("c1");
    let positions = get(&mock, "/v2/positions");
    assert_eq!(positions.as_array().expect("a JSON array").len(), 0);
}

#[test]
fn set_order_status_is_shown_by_lookup_and_list() {
    // R-119
    let mock = seeded();
    mock.set_order_status("c1", "replaced");
    assert_eq!(by_client_id(&mock, "c1")["status"], "replaced");
    assert_eq!(listed(&mock, "c1")["status"], "replaced");
}

#[test]
fn set_order_status_accepts_any_status_text() {
    // R-119
    let mock = seeded();
    mock.set_order_status("c1", "pending_cancel");
    assert_eq!(by_client_id(&mock, "c1")["status"], "pending_cancel");
}
