//! R-117: the mock refuses an oversized sell and a short of a non-shortable asset.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn post_order(mock: &MockAlpaca, id: &str, symbol: &str, side: &str, qty: &str) -> (u16, String) {
    let body = serde_json::json!({
        "symbol": symbol,
        "qty": qty,
        "side": side,
        "type": "market",
        "time_in_force": "day",
        "client_order_id": id,
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
    (status, resp.body_mut().read_to_string().expect("body"))
}

fn positions(mock: &MockAlpaca) -> Vec<serde_json::Value> {
    let mut resp = agent()
        .get(&format!("{}/v2/positions", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    let text = resp.body_mut().read_to_string().expect("body");
    serde_json::from_str::<serde_json::Value>(&text)
        .expect("json")
        .as_array()
        .expect("array")
        .clone()
}

fn qty_of(mock: &MockAlpaca, symbol: &str) -> Option<String> {
    positions(mock)
        .iter()
        .find(|p| p["symbol"] == symbol)
        .map(|p| p["qty"].as_str().expect("qty text").to_string())
}

fn long_spy() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    let (status, _) = post_order(&mock, "b1", "SPY", "buy", "10");
    assert_eq!(status, 200);
    mock.fill("b1", 10, "100");
    mock
}

#[test]
fn a_sell_larger_than_the_long_position_is_refused_with_403() {
    // R-117
    let mock = long_spy();
    let (status, body) = post_order(&mock, "s1", "SPY", "sell", "15");
    assert_eq!(status, 403);
    assert!(body.contains("insufficient qty"), "body was {body}");
}

#[test]
fn a_sell_within_the_long_position_is_accepted() {
    // R-117 boundary
    let mock = long_spy();
    let (status, _) = post_order(&mock, "s1", "SPY", "sell", "10");
    assert_eq!(status, 200);
}

#[test]
fn a_sell_of_a_non_shortable_asset_with_no_position_is_refused_with_403() {
    // R-117
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("XYZ", true, false, false);
    let (status, _) = post_order(&mock, "s1", "XYZ", "sell", "5");
    assert_eq!(status, 403);
}

#[test]
fn a_sell_of_a_shortable_asset_with_no_position_is_accepted_and_fills_short() {
    // R-117
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("TLT", true, true, true);
    let (status, _) = post_order(&mock, "s1", "TLT", "sell", "5");
    assert_eq!(status, 200);
    mock.fill("s1", 5, "90");
    assert_eq!(qty_of(&mock, "TLT").as_deref(), Some("-5"));
}
