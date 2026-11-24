//! R-115: scripted fills move order state, positions, cash and activities together.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn post_order(mock: &MockAlpaca, client_order_id: &str, side: &str, qty: &str) {
    let body = serde_json::json!({
        "symbol": "SPY",
        "qty": qty,
        "side": side,
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

fn order(mock: &MockAlpaca, client_order_id: &str) -> serde_json::Value {
    get(
        mock,
        &format!("/v2/orders:by_client_order_id?client_order_id={client_order_id}"),
    )
}

/// A decimal string with trailing zeros removed, so "100.00" equals "100".
fn norm(v: &serde_json::Value) -> String {
    let s = match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    post_order(&mock, "c1", "buy", "10");
    mock
}

fn spy_position(mock: &MockAlpaca) -> Option<serde_json::Value> {
    get(mock, "/v2/positions")
        .as_array()
        .expect("a JSON array")
        .iter()
        .find(|p| p["symbol"] == "SPY")
        .cloned()
}

#[test]
fn a_partial_fill_makes_the_order_partially_filled() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 4, "100");
    let o = order(&mock, "c1");
    assert_eq!(o["status"], "partially_filled");
    assert_eq!(norm(&o["filled_qty"]), "4");
}

#[test]
fn the_completing_fill_makes_the_order_filled() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 4, "100");
    mock.fill("c1", 6, "101");
    let o = order(&mock, "c1");
    assert_eq!(o["status"], "filled");
    assert_eq!(norm(&o["filled_qty"]), "10");
}

#[test]
fn positions_show_the_filled_quantity() {
    // R-115
    let mock = seeded();
    assert!(spy_position(&mock).is_none(), "no position before a fill");
    mock.fill("c1", 4, "100");
    assert_eq!(norm(&spy_position(&mock).expect("SPY")["qty"]), "4");
    mock.fill("c1", 6, "101");
    assert_eq!(norm(&spy_position(&mock).expect("SPY")["qty"]), "10");
}

#[test]
fn cash_falls_by_the_cost_of_each_fill() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 4, "100");
    assert_eq!(norm(&get(&mock, "/v2/account")["cash"]), "99600");
    mock.fill("c1", 6, "101");
    assert_eq!(norm(&get(&mock, "/v2/account")["cash"]), "98994");
}

#[test]
fn a_sell_fill_reduces_the_position_and_raises_cash() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 10, "100");
    post_order(&mock, "c2", "sell", "4");
    mock.fill("c2", 4, "110");
    assert_eq!(norm(&spy_position(&mock).expect("SPY")["qty"]), "6");
    // 100000 - 1000 + 440
    assert_eq!(norm(&get(&mock, "/v2/account")["cash"]), "99440");
}

#[test]
fn fill_activities_list_each_fill_with_distinct_ids() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 4, "100");
    mock.fill("c1", 6, "101");
    let v = get(&mock, "/v2/account/activities/FILL");
    let items = v.as_array().expect("a JSON array");
    assert_eq!(items.len(), 2);
    assert_ne!(items[0]["id"], items[1]["id"]);
    assert_eq!(norm(&items[0]["qty"]), "4");
    assert_eq!(norm(&items[0]["price"]), "100");
    assert_eq!(items[0]["type"], "partial_fill");
    assert_eq!(norm(&items[1]["qty"]), "6");
    assert_eq!(norm(&items[1]["price"]), "101");
    assert_eq!(items[1]["type"], "fill");
}

#[test]
fn no_activities_before_any_fill() {
    // R-115
    let mock = seeded();
    let v = get(&mock, "/v2/account/activities/FILL");
    assert!(v.as_array().expect("a JSON array").is_empty());
}

#[test]
fn fills_carry_an_increasing_rfc3339_utc_transaction_time() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 4, "100");
    mock.fill("c1", 6, "101");
    let v = get(&mock, "/v2/account/activities/FILL");
    let at = |i: usize| {
        let text = v[i]["transaction_time"].as_str().expect("a string");
        assert!(text.ends_with('Z'), "UTC: {text}");
        chrono::DateTime::parse_from_rfc3339(text).expect("RFC 3339")
    };
    assert!(at(1) > at(0));
}

#[test]
#[should_panic(expected = "cannot fill an order in status filled")]
fn filling_an_order_that_is_already_filled_panics() {
    // R-115
    let mock = seeded();
    mock.fill("c1", 10, "100");
    mock.fill("c1", 1, "100");
}
