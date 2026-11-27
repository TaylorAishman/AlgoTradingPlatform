//! R-441: the mock answers an order with an error and still holds it.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn post_order(mock: &MockAlpaca, client_order_id: &str) -> (u16, serde_json::Value) {
    let body = serde_json::json!({
        "symbol": "SPY",
        "qty": "10",
        "side": "buy",
        "type": "market",
        "time_in_force": "day",
        "client_order_id": client_order_id,
    })
    .to_string();
    let mut resp = agent()
        .post(&format!("{}/v2/orders", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .header("Content-Type", "application/json")
        .send(body)
        .expect("the request reaches the mock");
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().expect("body reads");
    (
        status,
        serde_json::from_str(&text).unwrap_or(serde_json::Value::Null),
    )
}

fn get(mock: &MockAlpaca, path: &str) -> (u16, serde_json::Value) {
    let mut resp = agent()
        .get(&format!("{}{}", mock.url(), path))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().expect("body reads");
    (
        status,
        serde_json::from_str(&text).unwrap_or(serde_json::Value::Null),
    )
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    mock
}

#[test]
fn reject_but_keep_holds_the_order_then_accepts_the_next() {
    // R-441
    let mock = seeded();
    mock.reject_but_keep_next_order(422);

    let (s1, _) = post_order(&mock, "c1");
    assert_eq!(s1, 422);

    let (lookup, body) = get(&mock, "/v2/orders:by_client_order_id?client_order_id=c1");
    assert_eq!(lookup, 200, "the order is held: {body}");
    assert_eq!(body["client_order_id"], "c1");
    assert_ne!(body["status"], "rejected");

    let (s2, b2) = post_order(&mock, "c2");
    assert_eq!(s2, 200, "the next POST succeeds: {b2}");
    assert_eq!(b2["client_order_id"], "c2");
}
