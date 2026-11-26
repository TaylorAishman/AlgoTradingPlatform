//! R-440: the mock refuses the next requests of one route.

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
fn reject_next_refuses_two_posts_then_accepts_the_third() {
    // R-440
    let mock = seeded();
    mock.reject_next("POST", "/v2/orders", 2, 422);

    let (s1, b1) = post_order(&mock, "c1");
    assert_eq!(s1, 422);
    assert!(b1["message"].is_string(), "body holds a message: {b1}");

    // An unrelated route between the refusals is served normally.
    let (acct, _) = get(&mock, "/v2/account");
    assert_eq!(acct, 200);

    let (s2, b2) = post_order(&mock, "c2");
    assert_eq!(s2, 422);
    assert!(b2["message"].is_string(), "body holds a message: {b2}");

    let (s3, b3) = post_order(&mock, "c3");
    assert_eq!(s3, 200, "third POST succeeds: {b3}");
    assert_eq!(b3["client_order_id"], "c3");
}

#[test]
fn refused_posts_create_no_order_and_are_recorded() {
    // R-440
    let mock = seeded();
    mock.reject_next("POST", "/v2/orders", 2, 422);
    post_order(&mock, "c1");
    post_order(&mock, "c2");

    let (_, list) = get(&mock, "/v2/orders?status=all");
    assert_eq!(list.as_array().expect("an array").len(), 0);
    let (lookup, _) = get(&mock, "/v2/orders:by_client_order_id?client_order_id=c1");
    assert_ne!(lookup, 200, "the refused order does not exist");

    let posts: Vec<_> = mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .collect();
    assert_eq!(posts.len(), 2, "both refused requests are recorded");

    post_order(&mock, "c3");
    let (_, list) = get(&mock, "/v2/orders?status=all");
    assert_eq!(list.as_array().expect("an array").len(), 1);
}

#[test]
fn reject_next_uses_the_given_status() {
    // R-440
    let mock = seeded();
    mock.reject_next("POST", "/v2/orders", 1, 403);
    assert_eq!(post_order(&mock, "c1").0, 403);
    assert_eq!(post_order(&mock, "c1").0, 200);
}

#[test]
fn reject_next_leaves_other_methods_on_the_same_path_alone() {
    // R-440
    let mock = seeded();
    mock.reject_next("POST", "/v2/orders", 1, 422);
    let (status, _) = get(&mock, "/v2/orders?status=all");
    assert_eq!(status, 200, "GET is not refused");
    assert_eq!(post_order(&mock, "c1").0, 422, "the POST is still refused");
}
