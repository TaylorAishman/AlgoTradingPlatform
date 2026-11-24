//! R-114: the mock accepts, deduplicates and returns orders (submit and lookup).

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn order_body(client_order_id: &str) -> String {
    serde_json::json!({
        "symbol": "SPY",
        "qty": "10",
        "side": "buy",
        "type": "market",
        "time_in_force": "day",
        "client_order_id": client_order_id,
    })
    .to_string()
}

fn post_order(mock: &MockAlpaca, client_order_id: &str) -> (u16, serde_json::Value) {
    let mut resp = agent()
        .post(&format!("{}/v2/orders", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .header("Content-Type", "application/json")
        .send(order_body(client_order_id))
        .expect("the request reaches the mock");
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().expect("body reads");
    let value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    (status, value)
}

fn lookup(mock: &MockAlpaca, client_order_id: &str) -> (u16, serde_json::Value) {
    let mut resp = agent()
        .get(&format!(
            "{}/v2/orders:by_client_order_id?client_order_id={}",
            mock.url(),
            client_order_id
        ))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().expect("body reads");
    let value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    (status, value)
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    mock
}

#[test]
fn a_new_order_is_accepted_with_zero_fill_and_an_id() {
    // R-114
    let mock = seeded();
    let (status, v) = post_order(&mock, "c1");
    assert_eq!(status, 200);
    assert_eq!(v["status"], "accepted");
    assert_eq!(v["filled_qty"], "0");
    assert!(v["id"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(v["client_order_id"], "c1");
    assert_eq!(v["symbol"], "SPY");
    assert_eq!(v["side"], "buy");
    assert_eq!(v["qty"], "10");
}

#[test]
fn the_same_client_order_id_posted_again_is_refused_422() {
    // R-114
    let mock = seeded();
    assert_eq!(post_order(&mock, "c1").0, 200);
    assert_eq!(post_order(&mock, "c1").0, 422);
}

#[test]
fn a_different_client_order_id_is_accepted_with_a_distinct_id() {
    // R-114
    let mock = seeded();
    let (_, first) = post_order(&mock, "c1");
    let (status, second) = post_order(&mock, "c2");
    assert_eq!(status, 200);
    assert_ne!(first["id"], second["id"]);
}

#[test]
fn lookup_by_client_order_id_returns_the_order() {
    // R-114
    let mock = seeded();
    let (_, posted) = post_order(&mock, "c1");
    let (status, v) = lookup(&mock, "c1");
    assert_eq!(status, 200);
    assert_eq!(v["client_order_id"], "c1");
    assert_eq!(v["id"], posted["id"]);
    assert_eq!(v["status"], "accepted");
    assert_eq!(v["filled_qty"], "0");
}

#[test]
fn lookup_of_an_unknown_client_order_id_is_404() {
    // R-114
    let mock = seeded();
    post_order(&mock, "c1");
    assert_eq!(lookup(&mock, "c2").0, 404);
}

#[test]
fn an_accepted_order_carries_an_rfc3339_utc_submitted_at_that_increases() {
    // R-114
    let mock = seeded();
    let (_, first) = post_order(&mock, "c1");
    let (_, second) = post_order(&mock, "c2");
    let parse = |v: &serde_json::Value| {
        let text = v["submitted_at"]
            .as_str()
            .expect("submitted_at is a string");
        assert!(text.ends_with('Z'), "UTC: {text}");
        chrono::DateTime::parse_from_rfc3339(text).expect("RFC 3339")
    };
    assert!(parse(&second) > parse(&first));
    let (_, looked_up) = lookup(&mock, "c1");
    assert_eq!(looked_up["submitted_at"], first["submitted_at"]);
}

fn post_raw(mock: &MockAlpaca, qty: &str, side: &str) -> u16 {
    let body = serde_json::json!({
        "symbol": "SPY", "qty": qty, "side": side, "type": "market",
        "time_in_force": "day", "client_order_id": "bad",
    })
    .to_string();
    agent()
        .post(&format!("{}/v2/orders", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .header("Content-Type", "application/json")
        .send(body)
        .expect("the request reaches the mock")
        .status()
        .as_u16()
}

#[test]
fn a_non_decimal_qty_or_an_unknown_side_is_refused_422_and_not_stored() {
    // R-114
    let mock = seeded();
    assert_eq!(post_raw(&mock, "ten", "buy"), 422);
    assert_eq!(post_raw(&mock, "10", "hold"), 422);
    assert_eq!(lookup(&mock, "bad").0, 404);
}
