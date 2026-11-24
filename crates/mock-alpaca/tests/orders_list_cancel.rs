//! R-114: the mock lists orders and cancels one by id.

use pt_mock_alpaca::server::MockAlpaca;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

fn post_order(mock: &MockAlpaca, client_order_id: &str) -> serde_json::Value {
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
    assert_eq!(resp.status().as_u16(), 200);
    let text = resp.body_mut().read_to_string().expect("body reads");
    serde_json::from_str(&text).expect("order json")
}

fn get(mock: &MockAlpaca, path_and_query: &str) -> (u16, serde_json::Value) {
    let mut resp = agent()
        .get(&format!("{}{}", mock.url(), path_and_query))
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

fn delete(mock: &MockAlpaca, id: &str) -> u16 {
    agent()
        .delete(&format!("{}/v2/orders/{}", mock.url(), id))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock")
        .status()
        .as_u16()
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    mock
}

#[test]
fn status_all_lists_a_submitted_order() {
    // R-114
    let mock = seeded();
    let posted = post_order(&mock, "c1");
    let (status, v) = get(&mock, "/v2/orders?status=all");
    assert_eq!(status, 200);
    let list = v.as_array().expect("a JSON array");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["client_order_id"], "c1");
    assert_eq!(list[0]["id"], posted["id"]);
}

#[test]
fn status_all_lists_every_order() {
    // R-114
    let mock = seeded();
    post_order(&mock, "c1");
    post_order(&mock, "c2");
    let (_, v) = get(&mock, "/v2/orders?status=all");
    assert_eq!(v.as_array().expect("a JSON array").len(), 2);
}

#[test]
fn deleting_an_order_returns_204_and_marks_it_canceled() {
    // R-114
    let mock = seeded();
    let posted = post_order(&mock, "c1");
    let id = posted["id"].as_str().expect("id").to_string();
    assert_eq!(delete(&mock, &id), 204);
    let (status, v) = get(&mock, "/v2/orders:by_client_order_id?client_order_id=c1");
    assert_eq!(status, 200);
    assert_eq!(v["status"], "canceled");
}

#[test]
fn a_canceled_order_still_appears_in_status_all_as_canceled() {
    // R-114
    let mock = seeded();
    let posted = post_order(&mock, "c1");
    delete(&mock, posted["id"].as_str().expect("id"));
    let (_, v) = get(&mock, "/v2/orders?status=all");
    let list = v.as_array().expect("a JSON array");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["status"], "canceled");
}

#[test]
fn deleting_an_unknown_order_id_is_404() {
    // R-114
    let mock = seeded();
    post_order(&mock, "c1");
    assert_eq!(delete(&mock, "no-such-id"), 404);
}

#[test]
fn orders_page_by_after_direction_and_limit_without_repeats() {
    // R-114
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_asset("SPY", true, true, true);
    for i in 0..5 {
        post_order(&mock, &format!("p{i}"));
    }
    let (_, first) = get(&mock, "/v2/orders?status=all&direction=asc&limit=3");
    let first = first.as_array().expect("array");
    assert_eq!(first.len(), 3);
    let last = first[2]["submitted_at"].as_str().expect("submitted_at");
    let encoded = last.replace(':', "%3A");
    let (_, second) = get(
        &mock,
        &format!("/v2/orders?status=all&direction=asc&limit=3&after={encoded}"),
    );
    let second = second.as_array().expect("array");
    assert_eq!(second.len(), 2);
    let ids: Vec<_> = first
        .iter()
        .chain(second.iter())
        .map(|o| o["id"].clone())
        .collect();
    let mut unique = ids.clone();
    unique.dedup();
    assert_eq!(ids.len(), 5);
    assert_eq!(unique.len(), 5, "no repeats across pages");
    assert_eq!(first[0]["client_order_id"], "p0");
    assert_eq!(second[1]["client_order_id"], "p4");
}
